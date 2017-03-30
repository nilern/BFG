use std::cmp::Ordering;
use std::io;
use std::io::{Read, Write};

use bytecode::Bytecode;

#[derive(Debug, Clone, Copy)]
pub enum Optcode {
    Inc,
    Dec,
    Add(u16),
    Sub(u16),

    SInc(i16),
    SDec(i16),
    SAdd(u8, i16),
    SSub(u8, i16),

    Jz(u16),
    Jnz(u16),

    Putc,
    Getc,
    Putci(i16),
    Getci(i16)
}

pub fn run(code: &[Optcode], data: &mut [u8]) -> io::Result<()> {
    let mut ip = 0usize;
    let mut dp = 0usize;

    use self::Optcode::*;

    while ip < code.len() {
        let instr = code[ip];
        ip += 1;
        match instr {
            Inc => dp += 1,
            Dec => dp -= 1,
            Add(n) => dp = (dp as isize + n as isize) as usize,
            Sub(n) => dp = (dp as isize - n as isize) as usize,

            SInc(offset) => {
                let i = (dp as isize + offset as isize) as usize;
                data[i] = data[i].wrapping_add(1);
            },
            SDec(offset) =>{
                let i = (dp as isize + offset as isize) as usize;
                data[i] = data[i].wrapping_sub(1);
            },
            SAdd(n, offset) => {
                let i = (dp as isize + offset as isize) as usize;
                data[i] = data[i].wrapping_add(n);
            },
            SSub(n, offset) =>{
                let i = (dp as isize + offset as isize) as usize;
                data[i] = data[i].wrapping_sub(n);
            },

            Jz(dest) => if data[dp] == 0 { ip = dest as usize; },
            Jnz(dest) => if data[dp] != 0 { ip = dest as usize; },

            Putc => { let _ = io::stdout().write(&data[dp..dp + 1])?; },
            Getc => { let _ = io::stdin().read(&mut data[dp..dp + 1])?; }
            Putci(offset) => {
                let i = (dp as isize + offset as isize) as usize;
                let _ = io::stdout().write(&data[i..i + 1])?;
            },
            Getci(offset) => {
                let i = (dp as isize + offset as isize) as usize;
                let _ = io::stdin().read(&mut data[i..i + 1])?;
            },
        }
    }

    Ok(())
}

pub fn convert(code: Vec<Bytecode>) -> Vec<Optcode> {
    use self::Optcode::*;

    let mut instrs = code.into_iter().peekable();
    let mut res = Vec::new();
    let mut labels = Vec::new();
    let mut dp_offset = 0i16;

    loop {
        match instrs.peek() {
            Some(&Bytecode::Inc) => {
                dp_offset += 1;
                instrs.next();
            },
            Some(&Bytecode::Dec) => {
                dp_offset -= 1;
                instrs.next();
            },
            Some(&Bytecode::SInc) | Some(&Bytecode::SDec) => {
                let mut value_offset = 0i8;
                loop {
                    match instrs.peek() {
                        Some(&Bytecode::SInc) => {
                            value_offset += 1;
                            instrs.next();
                        },
                        Some(&Bytecode::SDec) => {
                            value_offset -= 1;
                            instrs.next();
                        },
                        Some(&Bytecode::Inc) | Some(&Bytecode::Dec)
                        | Some(&Bytecode::Jz(_)) | Some(&Bytecode::Jnz(_))
                        | Some(&Bytecode::Putc)
                        | None => {
                            commit_write(&mut res, dp_offset, value_offset);
                            break;
                        },
                        Some(&Bytecode::Getc) => break,
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
            Some(&Bytecode::Putc) => {
                res.push(if dp_offset == 0 { Putc } else { Putci(dp_offset) });
                instrs.next();
            }
            Some(&Bytecode::Getc) => {
                res.push(if dp_offset == 0 { Getc } else { Getci(dp_offset) });
                instrs.next();
            },
            None => break
        }
    }

    res
}

fn commit_dp(res: &mut Vec<Optcode>, dp_offset: i16) {
    use self::Optcode::*;

    match dp_offset.cmp(&0) {
        Ordering::Greater => if dp_offset == 1 {
            res.push(Inc);
        } else {
            res.push(Add(dp_offset as u16));
        },
        Ordering::Less => if dp_offset == 1 {
            res.push(Dec);
        } else {
            res.push(Sub(-dp_offset as u16));
        },
        Ordering::Equal => ()
    }
}

fn commit_write(res: &mut Vec<Optcode>, dp_offset: i16, value_offset: i8) {
    use self::Optcode::*;

    match value_offset.cmp(&0) {
        Ordering::Greater => if value_offset == 1 {
            res.push(SInc(dp_offset));
        } else {
            res.push(SAdd(value_offset as u8, dp_offset));
        },
        Ordering::Less => if value_offset == -1 {
            res.push(SDec(dp_offset));
        } else {
            res.push(SSub(-value_offset as u8, dp_offset));
        },
        Ordering::Equal => ()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use std::fs::File;
    use std::io::Read;
    use parse::parse;

    #[bench]
    fn bench_optcode(b: &mut Bencher) {
        let mut f = File::open("bf/hello.b").expect("unable to open file");
        let mut src = String::new();
        f.read_to_string(&mut src).expect("error reading from file");
        let bc = parse(&src).unwrap();
        let oc = convert(bc);
        b.iter(|| run(&oc, &mut vec![0; 30_000]));
    }
}
