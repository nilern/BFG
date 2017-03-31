use std::io;
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy)]
pub enum Bytecode {
    PAdd(i16),
    DAdd(i8, i16),
    Jz(u16),
    Jnz(u16),
    Putc(i16),
    Getc(i16)
}

pub fn run(code: &[Bytecode], data: &mut [u8]) -> io::Result<()> {
    use bytecode::Bytecode::*;

    let mut ip = 0usize;
    let mut dp = 0usize;

    while ip < code.len() {
        let instr = code[ip];
        ip += 1;
        match instr {
            PAdd(i) => dp = (dp as isize + i as isize) as usize,
            DAdd(n, i) => {
                let dest = &mut data[(dp as isize + i as isize) as usize];
                *dest = (*dest as i8).wrapping_add(n) as u8;
            },
            Jz(dest) => if data[dp] == 0 { ip = dest as usize; },
            Jnz(dest) => if data[dp] != 0 { ip = dest as usize; },

            Putc(i) => {
                let index = (dp as isize + i as isize) as usize;
                let _ = io::stdout().write(&data[index..index + 1])?;
            },
            Getc(i) => {
                let index = (dp as isize + i as isize) as usize;
                let _ = io::stdin().read(&mut data[index..index + 1])?;
            }
        }
    }

    Ok(())
}

pub fn optimize(code: Vec<Bytecode>) -> Vec<Bytecode> {
    use bytecode::Bytecode::*;

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
            Some(&Bytecode::Jz(_)) => {
                commit_dp(&mut res, dp_offset);
                dp_offset = 0;

                res.push(Jz(0));
                labels.push(res.len() as u16);
                instrs.next();
            },
            Some(&Bytecode::Jnz(_)) => {
                commit_dp(&mut res, dp_offset);
                dp_offset = 0;

                let target = labels.pop().unwrap();
                res.push(Jnz(target));
                res[target as usize - 1] = Jz(res.len() as u16);
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

fn commit_dp(res: &mut Vec<Bytecode>, dp_offset: i16) {
    if dp_offset != 0 { res.push(Bytecode::PAdd(dp_offset)); }
}

fn commit_write(res: &mut Vec<Bytecode>, dp_offset: i16, value_offset: i8) {
    if value_offset != 0 { res.push(Bytecode::DAdd(value_offset, dp_offset)); }
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
        let code = parse(&src).unwrap();
        b.iter(|| run(&code, &mut vec![0; 30_000]));
    }

    #[bench]
    fn bench_bcopt_hello(b: &mut Bencher) {
        let mut f = File::open("bf/hello.b").expect("unable to open file");
        let mut src = String::new();
        f.read_to_string(&mut src).expect("error reading from file");
        let bc = parse(&src).unwrap();
        let oc = optimize(bc);
        b.iter(|| run(&oc, &mut vec![0; 30_000]));
    }
}
