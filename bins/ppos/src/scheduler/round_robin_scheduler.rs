use aarch64_cpu::registers::Writeable;
use alloc::{
    boxed::Box,
    collections::{LinkedList, VecDeque},
    sync::Arc,
};
use library::sync::mutex::Mutex;

use crate::{
    driver::timer,
    memory::{phys_binary_load_addr, phys_dram_start_addr},
};

use super::{
    current, switch_to,
    task::{StackInfo, Task, TaskState},
    Scheduler,
};

pub struct RoundRobinScheduler {
    run_queue: Mutex<VecDeque<Arc<Mutex<Task>>>>,
    wait_queue: Mutex<LinkedList<Arc<Mutex<Task>>>>,
}

impl RoundRobinScheduler {
    pub const fn new() -> Self {
        Self {
            run_queue: Mutex::new(VecDeque::new()),
            wait_queue: Mutex::new(LinkedList::new()),
        }
    }
}

impl RoundRobinScheduler {
    fn idle(&self) -> ! {
        loop {
            let mut i = 0;
            while i < {
                timer().disable_timer_interrupt();
                self.run_queue.lock().unwrap().len()
            } {
                {
                    let task_ref = self.run_queue.lock().unwrap()[i].clone();
                    let task = task_ref.lock().unwrap();
                    if task.state() == TaskState::Dead {
                        self.run_queue.lock().unwrap().remove(i);
                    }
                }
                timer().enable_timer_interrupt();
                i += 1;
            }
            self.schedule();
        }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn schedule(&self) -> *mut Task {
        // protect the scheduler from being interrupted
        timer().disable_timer_interrupt();
        if let Some(mut next_task) = self.run_queue.lock().unwrap().pop_front() {
            let next_task_state = next_task.lock().unwrap().state();
            let next = &mut *next_task.lock().unwrap() as *mut Task;
            self.run_queue.lock().unwrap().push_back(next_task);
            if next_task_state == TaskState::Dead {
                timer().enable_timer_interrupt();
                // if the task is dead, skip the job
                return self.schedule();
            }
            timer().enable_timer_interrupt();
            unsafe { switch_to(current(), next) }
        } else {
            timer().enable_timer_interrupt();
            panic!("No task to run!");
        }
    }

    fn add_task(&self, task: Task) {
        self.run_queue
            .lock()
            .unwrap()
            .push_back(Arc::new(Mutex::new(task)));
    }

    fn start_scheduler(&self) -> ! {
        let idle_task = Arc::new(Mutex::new(Task::new(StackInfo::new(
            phys_dram_start_addr() as *mut u8,
            phys_binary_load_addr() as *mut u8,
        ))));
        let idle_task_ptr = &*idle_task.lock().unwrap() as *const Task;
        {
            let mut run_queue = self.run_queue.lock().unwrap();
            // set the idle task as the first task, it will sync thread context automatically
            run_queue.push_front(idle_task);
            // set the idle task as current task
            aarch64_cpu::registers::TPIDR_EL1.set(idle_task_ptr as u64);
        }
        timer().set_timeout_raw(
            timer().tick_per_second() >> 5,
            Box::new(scheduler_timer_handler),
        );
        self.idle();
    }
}

fn scheduler_timer_handler() -> Result<(), &'static str> {
    let interval = timer().tick_per_second() >> 5;
    timer().set_timeout_raw(interval, Box::new(scheduler_timer_handler));
    ROUND_ROBIN_SCHEDULER.schedule();
    Ok(())
}

pub static ROUND_ROBIN_SCHEDULER: RoundRobinScheduler = RoundRobinScheduler::new();
