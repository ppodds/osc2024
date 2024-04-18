use core::array;

use alloc::sync::Arc;
use cpu::cpu::{disable_kernel_space_interrupt, enable_kernel_space_interrupt};
use library::sync::mutex::Mutex;

use crate::scheduler::task::Task;

pub type PIDNumber = usize;

#[derive(Debug, Clone, Copy)]
pub enum PIDType {
    PID,
    TGID,
    PGID,
    SID,
    Max,
}

#[derive(Debug, Clone)]
pub struct PID {
    number: PIDNumber,
    tasks: [Option<Arc<Mutex<Task>>>; PIDType::Max as usize],
}

impl PID {
    pub fn new() -> Self {
        unsafe {
            disable_kernel_space_interrupt();
            let res = Self {
                number: CURRENT_PID,
                tasks: [None, None, None, None],
            };
            CURRENT_PID += 1;
            enable_kernel_space_interrupt();
            res
        }
    }

    #[inline(always)]
    pub fn number(&self) -> PIDNumber {
        self.number
    }
}

// workaround
static mut CURRENT_PID: PIDNumber = 0;
