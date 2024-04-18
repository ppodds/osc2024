use core::arch::global_asm;

use alloc::sync::Arc;
use tock_registers::interfaces::Readable;

use library::sync::mutex::Mutex;

use self::task::Task;

pub mod round_robin_scheduler;
pub mod task;

global_asm!(include_str!("scheduler/switch_to.s"));

extern "C" {
    pub fn switch_to(prev: *mut Task, next: *mut Task) -> *mut Task;
}

#[inline(always)]
pub fn current() -> *mut Task {
    aarch64_cpu::registers::TPIDR_EL1.get() as *mut Task
}

pub trait Scheduler {
    fn schedule(&self) -> *mut Task;

    fn add_task(&self, task: Task);

    fn start_scheduler(&self) -> !;
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
}

static SCHEDULER: Mutex<&'static dyn Scheduler> = Mutex::new(&NullScheduler::new());

pub fn register_scheduler(scheduler: &'static dyn Scheduler) {
    *SCHEDULER.lock().unwrap() = scheduler;
}

pub fn scheduler() -> &'static dyn Scheduler {
    *SCHEDULER.lock().unwrap()
}
