use bytecode::Stmt;

#[derive(Debug)]
pub struct ParseError {
    index: usize
}

pub fn parse(code: &str) -> Result<Vec<Stmt>, ParseError> {
    use bytecode::Stmt::*;

    let mut res = Vec::new();
    let mut labels = Vec::new();
    let mut index = 0;

    for (i, c) in code.chars().enumerate() {
        index = i;
        match c {
            '>' => res.push(PAdd(1)),
            '<' => res.push(PAdd(-1)),
            '+' => res.push(DAdd(1, 0)),
            '-' => res.push(DAdd(-1, 0)),
            '[' => {
                res.push(Jz(0));
                labels.push((Jz(0), res.len()));
            },
            ']' => if let Some((Jz(_), target)) = labels.pop() {
                let diff = (res.len() - target + 1) as i16;
                res.push(Jnz(-diff));
                res[target - 1] = Jz(diff);
            } else {
                return Err(ParseError { index: index });
            },
            '.' => res.push(Putc(0)),
            ',' => res.push(Getc(0)),
            _ => ()
        }
    }

    if !labels.is_empty() {
        Err(ParseError { index: index })
    } else {
        Ok(res)
    }
}
