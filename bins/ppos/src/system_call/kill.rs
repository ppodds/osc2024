use cpu::cpu::{disable_kernel_space_interrupt, enable_kernel_space_interrupt};

use crate::pid::pid_manager;

pub fn kill(pid: i32) {
    unsafe { disable_kernel_space_interrupt() }
    if let Some(pid) = pid_manager().get_pid(pid as usize) {
        pid.lock()
            .unwrap()
            .pid_task()
            .unwrap()
            .lock()
            .unwrap()
            .kill();
    }
    unsafe { enable_kernel_space_interrupt() }
}
