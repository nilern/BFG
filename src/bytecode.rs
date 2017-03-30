use std::io;
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy)]
pub enum Bytecode {
    Inc,
    Dec,

    SInc,
    SDec,

    Jz(u16),
    Jnz(u16),

    Putc,
    Getc
}

pub fn run(code: &[Bytecode], data: &mut [u8]) -> io::Result<()> {
    let mut ip = 0;
    let mut dp = 0;

    use self::Bytecode::*;

    while ip < code.len() {
        let instr = code[ip];
        ip += 1;
        match instr {
            Inc => dp += 1,
            Dec => dp -= 1,

            SInc => data[dp] = data[dp].wrapping_add(1),
            SDec => data[dp] = data[dp].wrapping_sub(1),

            Jz(dest) => if data[dp] == 0 { ip = dest as usize; },
            Jnz(dest) => if data[dp] != 0 { ip = dest as usize; },

            Putc => { let _ = io::stdout().write(&data[dp..dp + 1])?; },
            Getc => { let _ = io::stdin().read(&mut data[dp..dp + 1])?; }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use std::fs::File;
    use std::io::Read;
    use parse::parse;

    #[bench]
    fn bench_bytecode(b: &mut Bencher) {
        let mut f = File::open("bf/hello.b").expect("unable to open file");
        let mut src = String::new();
        f.read_to_string(&mut src).expect("error reading from file");
        let code = parse(&src).unwrap();
        b.iter(|| run(&code, &mut vec![0; 30_000]));
    }
}
