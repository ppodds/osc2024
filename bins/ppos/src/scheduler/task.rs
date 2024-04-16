use alloc::boxed::Box;
use cpu::thread::Thread;

use crate::{memory::PAGE_SIZE, scheduler::current};

use super::scheduler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum TaskState {
    Running,
    Interruptable,
    Uninterruptable,
    Zombie,
    Stopped,
    Traced,
    Dead,
    Swapping,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StackInfo {
    pub top: *mut u8,
    pub bottom: *mut u8,
}

impl StackInfo {
    pub const fn new(top: *mut u8, bottom: *mut u8) -> Self {
        Self { top, bottom }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Task {
    thread: Thread,
    state: TaskState,
    stack: StackInfo,
}

impl Task {
    pub const fn new(stack: StackInfo) -> Self {
        Self {
            thread: Thread::new(),
            state: TaskState::Running,
            stack,
        }
    }

    pub fn from_job(job: fn() -> !) -> Self {
        // call into_raw to prevent the Box from being dropped
        let mut stack = Box::into_raw(Box::new([0; PAGE_SIZE]));
        let stack_bottom = (stack as usize + (unsafe { *stack }).len()) as *mut u8;
        let mut task = Self::new(StackInfo::new(stack as *mut u8, stack_bottom));
        task.thread.context.pc = job as u64;
        task.thread.context.sp = stack_bottom as u64;
        task
    }

    #[inline(always)]
    pub fn thread(&self) -> &Thread {
        &self.thread
    }

    #[inline(always)]
    pub fn state(&self) -> TaskState {
        self.state
    }

    #[inline(always)]
    pub fn stack(&self) -> StackInfo {
        self.stack
    }

    /**
     * Leave the kernel thread and return to the scheduler
     */
    pub fn exit(code: usize) -> ! {
        let current = unsafe { &mut *current() };
        current.state = TaskState::Dead;
        // let the idle task to clean up the task
        // we can't clean up the task here because the task is still running
        scheduler().schedule();
        panic!("Unreachable!")
    }
}
