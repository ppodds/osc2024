#[derive(Debug, Clone)]
#[repr(C)]
pub struct CPUContext {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub fp: u64,
    pub sp: u64,
    pub pc: u64,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Thread {
    pub context: CPUContext,
}

impl Thread {
    pub const fn new() -> Self {
        Self {
            context: CPUContext {
                x19: 0,
                x20: 0,
                x21: 0,
                x22: 0,
                x23: 0,
                x24: 0,
                x25: 0,
                x26: 0,
                x27: 0,
                x28: 0,
                fp: 0,
                sp: 0,
                pc: 0,
            },
        }
    }
}
