use std::io;
use std::io::{Write, Read};

#[derive(Debug, Clone, Copy)]
pub enum Bytecode {
    Inc,
    Dec,
    SInc,
    SDec,
    Jz(usize),
    Jnz(usize),
    Putc,
    Getc
}

#[derive(Debug)]
pub struct ParseError {
    index: usize
}

pub fn parse(code: &str) -> Result<Vec<Bytecode>, ParseError> {
    use self::Bytecode::*;

    let mut res = Vec::new();
    let mut labels = Vec::new();
    let mut index = 0;

    for (i, c) in code.chars().enumerate() {
        index = i;
        match c {
            '>' => res.push(Inc),
            '<' => res.push(Dec),
            '+' => res.push(SInc),
            '-' => res.push(SDec),
            '[' => {
                res.push(Jz(0));
                labels.push((Jz(0), res.len()));
            },
            ']' => if let Some((Jz(_), target)) = labels.pop() {
                res.push(Jnz(target));
                res[target - 1] = Jz(res.len());
            } else {
                return Err(ParseError { index: index });
            },
            '.' => res.push(Putc),
            ',' => res.push(Getc),
            _ => ()
        }
    }

    if !labels.is_empty() {
        Err(ParseError { index: index })
    } else {
        Ok(res)
    }
}

#[derive(Debug)]
pub struct VM {
    code: Vec<Bytecode>,
    ip: usize,
    data: Vec<u8>,
    dp: usize
}

impl VM {
    pub fn new(mem_size: usize, code: Vec<Bytecode>) -> VM {
        VM {
            code: code,
            ip: 0,
            data: vec![0; mem_size],
            dp: 0
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        use self::Bytecode::*;

        while self.ip < self.code.len() {
            let instr = self.code[self.ip];
            self.ip += 1;
            match instr {
                Inc => self.dp += 1,
                Dec => self.dp -= 1,
                SInc => self.data[self.dp] = self.data[self.dp].wrapping_add(1),
                SDec => self.data[self.dp] = self.data[self.dp].wrapping_sub(1),
                Jz(dest) => if self.data[self.dp] == 0 { self.ip = dest; },
                Jnz(dest) => if self.data[self.dp] != 0 { self.ip = dest; },
                Putc => { let _ = io::stdout().write(&self.data[self.dp..self.dp + 1])?; },
                Getc => { let _ = io::stdin().read(&mut self.data[self.dp..self.dp + 1])?; }
            }
        }

        Ok(())
    }
}
