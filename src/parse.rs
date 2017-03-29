use bytecode::Bytecode;

#[derive(Debug)]
pub struct ParseError {
    index: usize
}

pub fn parse(code: &str) -> Result<Vec<Bytecode>, ParseError> {
    use bytecode::Bytecode::*;

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
                labels.push((Jz(0), res.len() as u16));
            },
            ']' => if let Some((Jz(_), target)) = labels.pop() {
                res.push(Jnz(target));
                res[target as usize - 1] = Jz(res.len() as u16);
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
