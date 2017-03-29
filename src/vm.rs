use std::io;
use std::io::{Read, Write};

use bytecode::Bytecode;

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
            Add(n) => dp += n as usize,
            Sub(n) => dp -= n as usize,

            SInc => data[dp] = data[dp].wrapping_add(1),
            SDec => data[dp] = data[dp].wrapping_sub(1),
            SAdd(n) => data[dp] = data[dp].wrapping_add(n),
            SSub(n) => data[dp] = data[dp].wrapping_sub(n),

            Jz(dest) => if data[dp] == 0 { ip = dest as usize; },
            Jnz(dest) => if data[dp] != 0 { ip = dest as usize; },

            Putc => { let _ = io::stdout().write(&data[dp..dp + 1])?; },
            Getc => { let _ = io::stdin().read(&mut data[dp..dp + 1])?; }
        }
    }

    Ok(())
}
