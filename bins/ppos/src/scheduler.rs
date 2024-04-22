use core::arch::global_asm;

use aarch64_cpu::registers::{Writeable, ELR_EL1, SPSR_EL1, SP_EL0, TPIDR_EL0, TPIDR_EL1};
use alloc::sync::Arc;
use cpu::thread::CPUContext;
use tock_registers::interfaces::Readable;

use library::sync::mutex::Mutex;

use self::task::Task;

pub mod round_robin_scheduler;
pub mod task;

global_asm!(include_str!("scheduler/switch_to.s"));

extern "C" {
    fn cpu_switch_to(prev: *mut Task, next: *mut Task) -> *mut Task;
}

unsafe fn software_thread_switch(prev: *mut Task, next: *mut Task) {
    let prev = &mut *prev;
    prev.thread.software_thread_registers.tpidr_el1 = TPIDR_EL1.get();
    prev.thread.software_thread_registers.tpidr_el0 = TPIDR_EL0.get();
    prev.thread.elr_el1 = ELR_EL1.get();
    prev.thread.sp_el0 = SP_EL0.get();
    prev.thread.spsr_el1 = SPSR_EL1.get();

    TPIDR_EL0.set(next as u64);
    TPIDR_EL1.set(next as u64);
    let next = &*next;
    ELR_EL1.set(next.thread.elr_el1);
    SP_EL0.set(next.thread.sp_el0);
    SPSR_EL1.set(next.thread.spsr_el1);
}

pub unsafe fn switch_to(prev: *mut Task, next: *mut Task) -> *mut Task {
    software_thread_switch(prev, next);
    cpu_switch_to(prev, next)
}

#[inline(always)]
pub fn current() -> *mut Task {
    aarch64_cpu::registers::TPIDR_EL1.get() as *mut Task
}

pub trait Scheduler {
    fn schedule(&self) -> *mut Task;

    fn add_task(&self, task: Task);

    fn start_scheduler(&self) -> !;

    fn execute_task(&self, task: Task);
}

struct NullScheduler {}

impl NullScheduler {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Scheduler for NullScheduler {
    fn schedule(&self) -> *mut Task {
        unimplemented!()
    }

    fn add_task(&self, task: Task) {
        unimplemented!()
    }

    fn start_scheduler(&self) -> ! {
        unimplemented!()
    }

    fn execute_task(&self, task: Task) {
        unimplemented!()
    }
}

static SCHEDULER: Mutex<&'static dyn Scheduler> = Mutex::new(&NullScheduler::new());

pub fn register_scheduler(scheduler: &'static dyn Scheduler) {
    *SCHEDULER.lock().unwrap() = scheduler;
}

pub fn scheduler() -> &'static dyn Scheduler {
    *SCHEDULER.lock().unwrap()
}
