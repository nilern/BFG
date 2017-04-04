#![feature(test, plugin)]
#![plugin(dynasm)]

extern crate test;
extern crate rustyline;
extern crate libc;
extern crate dynasmrt;

use std::fs::File;
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::Editor;

mod parse;
mod bytecode;
mod aot;

use parse::parse;

// TODO: allow selecting bytecode, VM optimization levels
// MAYBE: debug switches

static mut DATA: [u8; 30_000] = [0; 30_000];

unsafe fn eval(src: &str, opt_level: usize) {
    match parse(src) {
        Ok(ir) => match opt_level {
            0 => {
                // let code = bytecode::assemble(ir.iter());
                // let bv = bytecode::vm();
                // bv.2(code.as_ptr(), code.len(), DATA.as_mut_ptr());
                // let _ = bytecode::run(&code, &mut DATA);
                aot::codegen(ir).1(DATA.as_mut_ptr());
            },
            1 => {
                let opt_ir = bytecode::optimize(ir);
                // let code = bytecode::assemble(opt_ir.iter());
                // let bv = bytecode::vm();
                // bv.2(code.as_ptr(), code.len(), DATA.as_mut_ptr());
                // let _ = bytecode::run(&code, &mut DATA);
                aot::codegen(opt_ir).1(DATA.as_mut_ptr());
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
            let mut src = String::new();
            f.read_to_string(&mut src).expect("error reading from file");
            unsafe { eval(&src, 1) };
        }
        _ => println!("Too many command line arguments.")
    }
}
