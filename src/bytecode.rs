#[derive(Debug, Clone, Copy)]
pub enum Bytecode {
    Inc,
    Dec,
    Add(u16),
    Sub(u16),

    SInc,
    SDec,
    SAdd(u8),
    SSub(u8),

    Jz(u16),
    Jnz(u16),

    Putc,
    Getc
}
