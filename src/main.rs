#![feature(test)]

extern crate rustyline;
extern crate test;

use std::fs::File;
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::Editor;

mod parse;
mod bytecode;

use parse::parse;

static mut DATA: [u8; 30_000] = [0; 30_000];

unsafe fn eval(code: &str, opt_level: usize) {
    match parse(code) {
        Ok(code) => match opt_level {
            0 => {
                let _ = bytecode::run(&code, &mut DATA);
            },
            1 => {
                let optcode = bytecode::optimize(code);
                let _ = bytecode::run(&optcode, &mut DATA);
            },
            _ => println!("Error: unsupported opt_level {}", opt_level)
        },
        Err(err) => println!("Error: {:?}", err)
    }
}

fn main() {
    let mut args = std::env::args();
    match args.len() {
        1 => {
            let mut rl = Editor::<()>::new();
            loop {
                let readline = rl.readline("bf> ");
                match readline {
                    Ok(line) => {
                        rl.add_history_entry(&line);
                        unsafe { eval(&line, 1) };
                    },
                    Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
        },
        2 => {
            let _ = args.next();
            let mut f = File::open(args.next().unwrap()).expect("unable to open file");
            let mut code = String::new();
            f.read_to_string(&mut code).expect("error reading from file");
            unsafe { eval(&code, 1) };
        }
        _ => println!("Too many command line arguments.")
    }
}
