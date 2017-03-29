extern crate rustyline;

use std::fs::File;
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::Editor;

mod bytecode;
mod parse;
mod vm;

use parse::parse;

fn naive(code: &str) {
    match parse(code) {
        Ok(code) => { let _ = vm::run(&code, &mut vec![0; 30_000]); },
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
                        naive(&line);
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
            naive(&code);
        }
        _ => println!("Too many command line arguments.")
    }
}
