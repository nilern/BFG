#![feature(test, proc_macro_hygiene)]

extern crate test;
extern crate rustyline;
extern crate libc;
extern crate dynasmrt;
extern crate dynasm;
extern crate rustc_serialize;
extern crate docopt;

use std::fs::File;
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use docopt::Docopt;

mod parse;
mod bytecode;
mod aot;

use parse::parse;

// MAYBE: debug switches

const USAGE: &'static str = "
Brainfsck Virtual Machine.

Usage: bfg [options] [<filename>]
       bfg --help

Options:
  --help, -h                    Show this message.
  --opt LEVEL, -O LEVEL         Set bytecode optimization level to LEVEL.
  --backend BACKEND -b BACKEND  Select backend. Valid values: rs, asm, aot.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_filename: Option<String>,
    flag_opt: Option<usize>,
    flag_backend: Option<Backend>
}

#[derive(RustcDecodable, Debug, Clone, Copy)]
enum Backend { Rs, Asm, AOT }

static mut DATA: [u8; 30_000] = [0; 30_000];

unsafe fn eval(src: &str, opt_level: usize, backend: Backend) {
    match parse(src) {
        Ok(mut ir) => {
            match opt_level {
                0 => (),
                1 => ir = bytecode::optimize(ir),
                _ => println!("Error: unsupported opt level {}", opt_level)
            }
            match backend {
                Backend::Rs => {
                    let code = bytecode::assemble(ir.iter());
                    let _ = bytecode::run(&code, &mut DATA);
                },
                Backend::Asm => {
                    let code = bytecode::assemble(ir.iter());
                    let bv = bytecode::vm();
                    bv.2(code.as_ptr(), code.len(), DATA.as_mut_ptr())
                },
                Backend::AOT => aot::codegen(ir).1(DATA.as_mut_ptr())
            }
        },
        Err(err) => println!("Error: {:?}", err)
    }
}

fn main() {
    match Docopt::new(USAGE).map(|d| d.help(true)).and_then(|d| d.decode()) {
        Ok(Args { arg_filename: Some(filename), flag_opt, flag_backend }) => {
            let mut f = File::open(filename).expect("unable to open file");
            let mut src = String::new();
            f.read_to_string(&mut src).expect("error reading from file");
            unsafe { eval(&src, flag_opt.unwrap_or(1), flag_backend.unwrap_or(Backend::AOT)) };
        },
        Ok(Args { arg_filename: None, flag_opt, flag_backend }) => {
            let mut rl = Editor::<()>::new();
            loop {
                let readline = rl.readline("bf> ");
                match readline {
                    Ok(line) => {
                        rl.add_history_entry(&line);
                        unsafe { eval(&line, flag_opt.unwrap_or(1),
                                      flag_backend.unwrap_or(Backend::AOT)) };
                    },
                    Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
        },
        Err(e) => e.exit()
    }
}
