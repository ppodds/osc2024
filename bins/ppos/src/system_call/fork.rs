use crate::scheduler::{current, scheduler};

pub fn fork() -> i32 {
    let current = unsafe { &*current() };
    let child = current.fork();
    let child_pid = child.pid();
    scheduler().add_task(child);
    if current.pid() == child_pid {
        0
    } else {
        child_pid as i32
    }
}
