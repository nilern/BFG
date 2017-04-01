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
                            value_offset += n;
                            instrs.next();
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
                labels.push(res.len() as i16);
                instrs.next();
            },
            Some(&Jnz(_)) => {
                commit_dp(&mut res, dp_offset);
                dp_offset = 0;

                let target = labels.pop().unwrap();
                let diff = res.len() as i16 - target + 1;
                res.push(Jnz(-diff));
                res[target as usize - 1] = Jz(diff);
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

pub fn vm() -> (ExecutableBuffer, extern fn(*const i32, usize, *mut u8)) {
    let mut ops = x64::Assembler::new();
    let jump_table = ops.new_dynamic_label();

    dynasm!(ops
        ; .alias ip, rbx
        ; .alias ie, rbp
        ; .alias dp, r12

        ; .alias instr, r13
        ; .alias opcode, r14
        ; .alias offset, r15

        ; .alias f, rcx
    );
    let vm_fn = ops.offset();
    dynasm!(ops
        ; sub rsp, 48
        ; mov [rsp + 48], rbp
        ; mov [rsp + 32], rbx
        ; mov [rsp + 24], r12
        ; mov [rsp + 16], r13
        ; mov [rsp + 8], r14
        ; mov [rsp], r15

        ; mov ip, rdi
        ; mov ie, rdi
        ; sal rsi, 2
        ; add ie, rsi
        ; mov dp, rdx

        // if ip >= ie { goto end; }
        ; cmp ip, ie
        ; jge ->end

        ; movsx instr, DWORD [ip] // instr = *ip
        // offset = instr >> ISHIFT
        ; mov offset, instr
        ; sar offset, ISHIFT as _
        // opcode = instr & 0xff
        ; mov opcode, instr
        ; and opcode, 0xff

        ; ->more: // loop entry
        ; add ip, 4 // ip = ip.offset(1)

        ; lea rax, [=>jump_table]
        ; shl opcode, 3
        ; add rax, opcode
        ; jmp rax

        ; ->padd:
        ; add dp, offset
        ; jmp ->tail

        ; ->dadd:
        ; mov rax, instr
        ; and rax, 0xff00
        ; sar rax, NSHIFT as _
        ; add BYTE [dp + offset], al
        ; jmp ->tail

        ; ->jz:
        ; cmp BYTE [dp], 0
        ; jne ->tail
        ; sal offset, 2
        ; add ip, offset
        ; jmp ->tail

        ; ->jnz:
        ; cmp BYTE [dp], 0
        ; je ->tail
        ; sal offset, 2
        ; add ip, offset
        ; jmp ->tail

        ; ->putc:
        ; mov rdi, [dp + offset]
        ; mov f, QWORD libc::putchar as _
        ; call f
        ; jmp ->tail

        ; ->getc:
        ; mov f, QWORD libc::getchar as _
        ; call f
        ; mov [dp + offset], rax
        ; jmp ->tail

        ; ->tail:
        ; cmp ip, ie
        ; jge ->end

        ; movsx instr, DWORD [ip] // instr = *ip
        // offset = instr >> ISHIFT
        ; mov offset, instr
        ; sar offset, ISHIFT as _
        // opcode = instr & 0xff
        ; mov opcode, instr
        ; and opcode, 0xff
        ; jmp ->more

        ; ->end:
        ; mov r15, [rsp]
        ; mov r14, [rsp + 8]
        ; mov r13, [rsp + 16]
        ; mov r12, [rsp + 24]
        ; mov rbx, [rsp + 32]
        ; mov rbp, [rsp + 48]
        ; add rsp, 48
        ; ret

        ; .align 8
        ; =>jump_table
        ; jmp ->padd
        ; .align 8
        ; jmp ->dadd
        ; .align 8
        ; jmp ->jz
        ; .align 8
        ; jmp ->jnz
        ; .align 8
        ; jmp ->putc
        ; .align 8
        ; jmp ->getc
    );

    let buf = ops.finalize().unwrap();
    let f = unsafe { mem::transmute(buf.ptr(vm_fn)) };
    (buf, f)
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
        b.iter(|| run(&code, &mut vec![0; 30_000]));
    }

    #[bench]
    fn bench_bcopt_hello(b: &mut Bencher) {
        let mut f = File::open("bf/hello.b").expect("unable to open file");
        let mut src = String::new();
        f.read_to_string(&mut src).expect("error reading from file");
        let ir = parse(&src).unwrap();
        let opt_ir = optimize(ir);
        let code = assemble(opt_ir.iter());
        b.iter(|| run(&code, &mut vec![0; 30_000]));
    }
}
