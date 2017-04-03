use std::io;
use std::io::{Read, Write};
use std::mem;
use dynasmrt::{DynasmApi, DynasmLabelApi, ExecutableBuffer};
use dynasmrt::x64;
use libc;

#[derive(Debug, Clone, Copy)]
pub enum Stmt {
    PAdd(i16),
    DAdd(i8, i16),
    Jz(i16),
    Jnz(i16),
    Putc(i16),
    Getc(i16)
}

impl Stmt {
    pub fn opcode(self) -> Opcode {
        use self::Stmt::*;
        match self {
            PAdd(_) => Opcode::PAdd,
            DAdd(_, _) => Opcode::DAdd,
            Jz(_) => Opcode::Jz,
            Jnz(_) => Opcode::Jnz,
            Putc(_) => Opcode::Putc,
            Getc(_) => Opcode::Getc
        }
    }
}

pub fn optimize(code: Vec<Stmt>) -> Vec<Stmt> {
    use bytecode::Stmt::*;

    let mut instrs = code.into_iter().peekable();
    let mut res = Vec::new();
    let mut labels = Vec::new();
    let mut dp_offset = 0i16;

    loop {
        match instrs.peek() {
            Some(&PAdd(i)) => {
                dp_offset += i;
                instrs.next();
            }
            Some(&DAdd(_, 0)) => {
                let mut value_offset = 0i8;
                loop {
                    match instrs.peek() {
                        Some(&DAdd(n, 0)) => {
                            match value_offset.checked_add(n) {
                                Some(nv_offset) => {
                                    value_offset = nv_offset;
                                    instrs.next();
                                },
                                None => {
                                    commit_write(&mut res, dp_offset, value_offset);
                                    value_offset = 0;
                                }
                            }
                        },
                        Some(&PAdd(_)) | Some(&Jz(_)) | Some(&Jnz(_)) | Some(&Putc(_)) | None => {
                            commit_write(&mut res, dp_offset, value_offset);
                            break;
                        },
                        Some(&Getc(_)) => break,
                        _ => unreachable!()
                    }
                }
            },
            Some(&Jz(_)) => {
                commit_dp(&mut res, dp_offset);
                dp_offset = 0;

                res.push(Jz(0));
                labels.push(res.len());
                instrs.next();
            },
            Some(&Jnz(_)) => {
                commit_dp(&mut res, dp_offset);
                dp_offset = 0;

                let target = labels.pop().unwrap();
                let diff = (res.len() - target + 1) as i16;
                res.push(Jnz(-diff));
                res[target - 1] = Jz(diff);
                instrs.next();
            },
            Some(&Putc(0)) => {
                res.push(Putc(dp_offset));
                instrs.next();
            }
            Some(&Getc(0)) => {
                res.push(Getc(dp_offset));
                instrs.next();
            },
            None => break,
            _ => unreachable!()
        }
    }

    res
}

fn commit_dp(res: &mut Vec<Stmt>, dp_offset: i16) {
    if dp_offset != 0 { res.push(Stmt::PAdd(dp_offset)); }
}

fn commit_write(res: &mut Vec<Stmt>, dp_offset: i16, value_offset: i8) {
    if value_offset != 0 { res.push(Stmt::DAdd(value_offset, dp_offset)); }
}

#[derive(Debug)]
pub enum Opcode {
    PAdd = 0,
    DAdd = 1,
    Jz = 2,
    Jnz = 3,
    Putc = 4,
    Getc = 5
}

const ISHIFT: u32 = 16;
const NSHIFT: u32 = 8;

pub fn assemble<'a, I>(code: I) -> Vec<i32> where I: Iterator<Item=&'a Stmt> {
    use self::Stmt::*;

    let mut res = Vec::new();

    for stmt in code {
        res.push(match stmt {
            &PAdd(i) | &Putc(i) | &Getc(i) =>
                stmt.opcode() as i32
                | (i as i32) << ISHIFT,
            &DAdd(n, i) =>
                stmt.opcode() as i32
                | (i as i32) << ISHIFT
                | ((n as u16) << NSHIFT) as i32,
            &Jz(dest) | &Jnz(dest) =>
                stmt.opcode() as i32
                | (dest as i32) << ISHIFT,
        });
    }

    res
}

pub fn run(code: &[i32], data: &mut [u8]) -> io::Result<()> {
    use self::Opcode::*;

    let mut ip = 0usize;
    let mut dp = 0usize;

    if ip < code.len() {
        let mut instr = code[ip];
        let mut opcode: Opcode = unsafe { mem::transmute((instr & 0xff) as u8) };
        let mut offset = instr >> ISHIFT;

        loop {
            ip += 1;

            match opcode {
                PAdd => dp = (dp as isize + offset as isize) as usize,
                DAdd => {
                    let i = dp as isize + offset as isize;
                    let n = instr as i16 >> NSHIFT;
                    let dest = &mut data[i as usize];
                    *dest = (*dest as i8).wrapping_add(n as i8) as u8;
                },

                Jz => if data[dp] == 0 { ip = (ip as isize + offset as isize) as usize; },
                Jnz => if data[dp] != 0 { ip = (ip as isize + offset as isize) as usize; },

                Putc => {
                    let i = (dp as isize + offset as isize) as usize;
                    let _ = io::stdout().write(&data[i..i + 1])?;
                },
                Getc => {
                    let i = (dp as isize + offset as isize) as usize;
                    let _ = io::stdin().read(&mut data[i..i + 1])?;
                }
            }

            if ip >= code.len() {
                break;
            }

            instr = code[ip];
            opcode = unsafe { mem::transmute((instr & 0xff) as u8) };
            offset = instr >> ISHIFT;
        }
    }

    Ok(())
}

// FIXME: flush stdout
pub fn vm() -> (ExecutableBuffer, Vec<usize>, extern fn(*const i32, usize, *mut u8)) {
    let mut ops = x64::Assembler::new();
    let jump_table = vec![0; 6];

    macro_rules! decode_dispatch {
        () => {dynasm!(ops
            ; cmp ip, ie
            ; jge ->end // halt

            // Decode most of instruction:
            ; movsx instr, DWORD [ip] // instr = *ip
            // offset = instr >> ISHIFT
            ; mov offset, instr
            ; sar offset, ISHIFT as _
            // opcode = instr & 0xff
            ; movzx opcode, al

            ; add ip, 4 // ip = ip.offset(1)

            // Indirect jump:
            ; mov rdi, QWORD jump_table.as_ptr() as _
            ; lea rdi, [rdi + opcode*8]
            ; jmp QWORD [rdi]
        )}
    }

    dynasm!(ops
        ; .alias ip, r12
        ; .alias ie, r13
        ; .alias dp, r14
        ; .alias offset, rbx

        ; .alias instr, rax
        ; .alias opcode, rcx
        ; .alias f, rax
    );
    let vm_fn = ops.offset();
    dynasm!(ops
        // Set up stack frame and save callee-save registers:
        ; push rbp
        ; mov rbp, rsp
        ; push rbx
        ; push r12
        ; push r13
        ; push r14

        // Fill jump table:
        ; mov rax, QWORD jump_table.as_ptr() as _
        ; lea rcx, [->padd]
        ; mov [rax], rcx
        ; add rax, 8
        ; lea rcx, [->dadd]
        ; mov [rax], rcx
        ; add rax, 8
        ; lea rcx, [->jz]
        ; mov [rax], rcx
        ; add rax, 8
        ; lea rcx, [->jnz]
        ; mov [rax], rcx
        ; add rax, 8
        ; lea rcx, [->putc]
        ; mov [rax], rcx
        ; add rax, 8
        ; lea rcx, [->getc]
        ; mov [rax], rcx

        // Setup variables based on args:
        ; mov ip, rdi
        ; mov ie, rdi
        ; sal rsi, 2
        ; add ie, rsi
        ; mov dp, rdx

        ;; decode_dispatch!()

        ; ->padd:
        ; add dp, offset
        ;; decode_dispatch!()

        ; ->dadd:
        ; and instr, 0xff00
        ; shr instr, NSHIFT as _
        ; add BYTE [dp + offset], al
        ;; decode_dispatch!()

        ; ->jz:
        ; cmp BYTE [dp], 0
        ; jne >tail
        ; lea ip, [ip + offset*4]
        ; tail:
        ;; decode_dispatch!()

        ; ->jnz:
        ; cmp BYTE [dp], 0
        ; je >tail
        ; lea ip, [ip + offset*4]
        ; tail:
        ;; decode_dispatch!()

        ; ->putc:
        ; mov rdi, [dp + offset]
        ; mov f, QWORD libc::putchar as _
        ; call f
        ;; decode_dispatch!()

        ; ->getc:
        ; mov f, QWORD libc::getchar as _
        ; call f
        ; mov [dp + offset], rax
        ;; decode_dispatch!()

        ; ->end:
        // Tear down stack frame and restore callee-save registers:
        ; pop r14
        ; pop r13
        ; pop r12
        ; pop rbx
        ; pop rbp
        ; ret
    );

    let buf = ops.finalize().unwrap();
    let f = unsafe { mem::transmute(buf.ptr(vm_fn)) };
    (buf, jump_table, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use std::fs::File;
    use std::io::Read;
    use parse::parse;

    #[bench]
    fn bench_naive_hello(b: &mut Bencher) {
        let mut f = File::open("bf/hello.b").expect("unable to open file");
        let mut src = String::new();
        f.read_to_string(&mut src).expect("error reading from file");

        let ir = parse(&src).unwrap();
        let code = assemble(ir.iter());
        let bv = vm();
        b.iter(|| bv.2(code.as_ptr(), code.len(), vec![0; 30_000].as_mut_ptr()));
        println!("done");
    }

    #[bench]
    fn bench_bcopt_hello(b: &mut Bencher) {
        let mut f = File::open("bf/hello.b").expect("unable to open file");
        let mut src = String::new();
        f.read_to_string(&mut src).expect("error reading from file");

        let ir = parse(&src).unwrap();
        let opt_ir = optimize(ir);
        let code = assemble(opt_ir.iter());
        let bv = vm();
        b.iter(|| bv.2(code.as_ptr(), code.len(), vec![0; 30_000].as_mut_ptr()));
        println!("done");
    }
}
