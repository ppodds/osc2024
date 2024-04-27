use core::arch::asm;

use aarch64_cpu::registers::Readable;
use aarch64_cpu::registers::SP_EL0;
use alloc::{boxed::Box, sync::Arc};
use cpu::cpu::run_user_code;
use cpu::thread::Thread;
use library::sync::mutex::Mutex;

use crate::{
    memory::PAGE_SIZE,
    pid::{PIDNumber, PID},
    scheduler::current,
};

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
    pub thread: Thread,
    state: TaskState,
    kernel_stack: StackInfo,
    user_stack: StackInfo,
    pid: Arc<Mutex<PID>>,
}

impl Task {
    pub fn new(stack: StackInfo) -> Self {
        Self {
            thread: Thread::new(),
            state: TaskState::Running,
            kernel_stack: stack,
            user_stack: StackInfo::new(core::ptr::null_mut(), core::ptr::null_mut()),
            pid: Arc::new(Mutex::new(PID::new())),
        }
    }

    pub fn from_job(job: fn() -> !) -> Self {
        // call into_raw to prevent the Box from being dropped
        let mut stack = Box::into_raw(Box::new([0_u8; 1024 * PAGE_SIZE]));
        let stack_bottom = (stack as usize + (unsafe { *stack }).len()) as *mut u8;
        let mut task = Self::new(StackInfo::new(stack as *mut u8, stack_bottom));
        task.thread.context.pc = job as u64;
        task.thread.context.sp = stack_bottom as u64;
        task
    }

    /// Fork a new task from the current task and return the child task pointer
    pub fn fork(&self, caller_sp: usize) -> *mut Task {
        // save return address
        let mut return_addr: usize = 0;
        unsafe { asm!("mov {}, lr", out(reg) return_addr) };

        let has_user_stack = self.user_stack.top != core::ptr::null_mut();
        // allocate a new stack for the child task
        let mut kernel_stack = Box::into_raw(Box::new([0_u8; 1024 * PAGE_SIZE]));
        let kernel_stack_bottom =
            (kernel_stack as usize + (unsafe { *kernel_stack }).len()) as *mut u8;
        let kernel_used_stack_len = self.kernel_stack.bottom as u64 - caller_sp as u64;
        // copy the stack from the parent task to the child task
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.kernel_stack.bottom.sub(kernel_used_stack_len as usize),
                kernel_stack_bottom.sub(kernel_used_stack_len as usize),
                kernel_used_stack_len as usize,
            );
        }
        // create a child task
        let mut task = Self::new(StackInfo::new(kernel_stack as *mut u8, kernel_stack_bottom));
        if has_user_stack {
            let mut user_stack = Box::into_raw(Box::new([0_u8; PAGE_SIZE]));
            let user_stack_bottom =
                (user_stack as usize + (unsafe { *user_stack }).len()) as *mut u8;
            task.user_stack = StackInfo::new(user_stack as *mut u8, user_stack_bottom);
            let user_used_stack_len = self.user_stack.bottom as u64 - SP_EL0.get();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.user_stack.bottom.sub(user_used_stack_len as usize),
                    task.user_stack.bottom.sub(user_used_stack_len as usize),
                    user_used_stack_len as usize,
                );
            }
            task.thread.sp_el0 = user_stack_bottom as u64 - user_used_stack_len;
        };
        // copy the current thread context
        // the registers are stored in the stack in compiler generated function prologue
        unsafe {
            asm!("mov x0, {}
            ldp {}, {}, [x0, -16]
            ldp {}, {}, [x0, -32]
            ldp {}, {}, [x0, -48]
            ldp {}, {}, [x0, -64]
            ldp {}, {}, [x0, -80]
            ldr {}, [x0, -88]", in(reg) caller_sp, out(reg) task.thread.context.x20, out(reg) task.thread.context.x19, out(reg) task.thread.context.x22, out(reg) task.thread.context.x21, out(reg) task.thread.context.x24, out(reg) task.thread.context.x23, out(reg) task.thread.context.x26, out(reg) task.thread.context.x25, out(reg) task.thread.context.x28, out(reg) task.thread.context.x27, out(reg) task.thread.context.fp);
        }
        // child thread will jump to child_entry function and use ret to return to the caller
        let child_entry: fn() = || unsafe {
            asm!(
                "ldr x0, [sp, -8]
        ldr lr, [sp, -16]
        mov sp, x0
        mov x0, {}", in(reg) current() as usize
            )
        };
        task.thread.context.pc = child_entry as u64;
        // set the new kernel stack pointer
        // the new kernel stack
        task.thread.context.sp = kernel_stack_bottom as u64 - kernel_used_stack_len;
        unsafe {
            // place the caller sp at sp - 8
            core::ptr::write(
                (task.kernel_stack.bottom.sub(kernel_used_stack_len as usize) as *mut usize).sub(1),
                task.kernel_stack.bottom as usize - kernel_used_stack_len as usize,
            );
            // place the return address at sp - 16
            core::ptr::write(
                (task.kernel_stack.bottom.sub(kernel_used_stack_len as usize) as *mut usize).sub(2),
                return_addr,
            );
        }
        let task = Arc::new(Mutex::new(task));
        let task_ptr = &mut *task.lock().unwrap() as *mut Task;
        scheduler().add_task(task);
        task_ptr
    }

    #[inline(always)]
    pub fn state(&self) -> TaskState {
        self.state
    }

    #[inline(always)]
    pub fn stack(&self) -> StackInfo {
        self.kernel_stack
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

    #[inline(always)]
    pub fn pid(&self) -> PIDNumber {
        self.pid.lock().unwrap().number()
    }

    pub fn run_user_program(&mut self, user_program: *const fn() -> !) {
        let code_start = user_program as u64;
        const USER_STACK_SIZE: usize = 4096;
        let stack_top = Box::into_raw(Box::new([0_u8; USER_STACK_SIZE])) as u64;
        let stack_bottom = stack_top + USER_STACK_SIZE as u64;
        self.user_stack.top = stack_top as *mut u8;
        self.user_stack.bottom = stack_bottom as *mut u8;
        unsafe { run_user_code(stack_bottom, code_start) };
    }
}
