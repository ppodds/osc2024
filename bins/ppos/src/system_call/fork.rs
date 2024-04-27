use core::arch::asm;

use crate::scheduler::current;

pub fn fork() -> i32 {
    let mut sp = 0;
    unsafe {
        asm!(
            "mov {}, sp",
            out(reg) sp,
        );
    }
    let child = unsafe { &*current() }.fork(sp);
    if current() == child {
        0
    } else {
        (unsafe { &*child }).pid() as i32
    }
}
