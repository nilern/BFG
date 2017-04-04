use std::mem;
use libc;
use dynasmrt::{DynasmApi, DynasmLabelApi, ExecutableBuffer};
use dynasmrt::x64;

use bytecode::Stmt;

pub fn codegen(ir: Vec<Stmt>) -> (ExecutableBuffer, extern fn(*mut u8)) {
    use bytecode::Stmt::*;

    let mut ops = x64::Assembler::new();
    let mut loops = Vec::new();

    let entry = ops.offset();
    dynasm!(ops
        ; push rbp
        ; mov rbp, rsp
        ; push rbx
        ; push r12
        ; push r13
        ; push r14

        ; mov dp, rdi
    );

    for stmt in ir {
        match stmt {
            PAdd(i) => dynasm!(ops
                ; add dp, DWORD i as _
            ),
            DAdd(n, i) => dynasm!(ops
                ; add BYTE [DWORD i as _ + dp], n
            ),
            Jz(_) => {
                let loop_begin = ops.new_dynamic_label();
                let loop_end = ops.new_dynamic_label();
                loops.push((loop_begin, loop_end));
                dynasm!(ops
                    ; cmp BYTE [dp], 0
                    ; je =>loop_end
                    ; =>loop_begin
                );
            },
            Jnz(_) => {
                let (loop_begin, loop_end) = loops.pop().unwrap();
                dynasm!(ops
                    ; cmp BYTE [dp], 0
                    ; jne =>loop_begin
                    ; =>loop_end
                );
            },
            Putc(i) => dynasm!(ops
                ; mov rdi, [DWORD i as _ + dp]
                ; mov f, QWORD libc::putchar as _
                ; call f
            ),
            Getc(i) => dynasm!(ops
                ; mov f, QWORD libc::getchar as _
                ; call f
                ; mov [DWORD i as _ + dp], rax
            )
        }
    }

    dynasm!(ops
        ; pop r14
        ; pop r13
        ; pop r12
        ; pop rbx
        ; pop rbp
        ; ret
    );

    let buf = ops.finalize().unwrap();
    let f = unsafe { mem::transmute(buf.ptr(entry)) };
    (buf, f)
}
