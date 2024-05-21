use core::arch::asm;
use core::arch::global_asm;
use core::mem::size_of;

use aarch64_cpu::registers::Readable;
use aarch64_cpu::registers::Writeable;
use aarch64_cpu::registers::SP_EL0;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use bsp::memory::GPU_MEMORY_MMIO_BASE;
use bsp::memory::GPU_MEMORY_MMIO_SIZE;
use cpu::cpu::disable_kernel_space_interrupt;
use cpu::cpu::enable_kernel_space_interrupt;
use cpu::cpu::run_user_code;
use cpu::thread::CPUContext;
use cpu::thread::Thread;
use library::sync::mutex::Mutex;

use crate::memory::paging::memory_mapping::MemoryExecutePermission;
use crate::memory::paging::memory_mapping::MemoryMapping;
use crate::memory::paging::page::MemoryAccessPermission;
use crate::memory::paging::page::MemoryAttribute;
use crate::memory::phys_to_virt;
use crate::memory::round_down;
use crate::memory::round_up;
use crate::memory::virt_to_phys;
use crate::pid::pid_manager;
use crate::signal;
use crate::signal::Signal;
use crate::{
    memory::PAGE_SIZE,
    pid::{PIDNumber, PID},
    scheduler::current,
};

use super::scheduler;

global_asm!(include_str!("store_context.s"));
global_asm!(include_str!("load_context.s"));
global_asm!(include_str!("signal_handler_wrapper.s"));

extern "C" {
    pub fn store_context(context: *mut CPUContext);
    pub fn load_context(context: *const CPUContext);
    fn signal_handler_wrapper();
}

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
#[derive(Debug)]
pub struct Task {
    pub thread: Thread,
    state: TaskState,
    kernel_stack: StackInfo,
    user_stack: StackInfo,
    signal_stack: StackInfo,
    pid: Rc<Mutex<PID>>,
    signal_handlers: [Option<fn() -> !>; Signal::Max as usize],
    /// The signals that are pending for the task, represented as a bitfield.
    pending_signals: u32,
    /// Whether the task is currently handling a signal.
    /// This is used to prevent signal handlers from being reentrant.
    doing_signal: bool,
    /// The context that was saved when the task do a user defined signal handler.
    signal_saved_context: Option<(u64, CPUContext)>,
    /// user space page table
    memory_mapping: Rc<MemoryMapping>,
    /// user code
    user_code: Option<Rc<Box<[u8]>>>,
}

impl Task {
    pub const USER_STACK_SIZE: usize = PAGE_SIZE * 4;
    pub const USER_STACK_START: usize = 0xffff_ffff_b000;
    pub const USER_STACK_END: usize = Self::USER_STACK_START + Self::USER_STACK_SIZE;
    pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 1024;
    const SIGNAL_HANDLER_WRAPPER_SHARE_START: usize = Self::USER_STACK_START - PAGE_SIZE;

    pub fn new(stack: StackInfo) -> Self {
        Self {
            thread: Thread::new(),
            state: TaskState::Running,
            kernel_stack: stack,
            user_stack: StackInfo::new(core::ptr::null_mut(), core::ptr::null_mut()),
            signal_stack: StackInfo::new(core::ptr::null_mut(), core::ptr::null_mut()),
            pid: pid_manager().new_pid(),
            signal_handlers: [None; Signal::Max as usize],
            pending_signals: 0,
            doing_signal: false,
            signal_saved_context: None,
            memory_mapping: Rc::new(MemoryMapping::new()),
            user_code: None,
        }
    }

    pub fn from_job(job: fn() -> !) -> Self {
        // call into_raw to prevent the Box from being dropped
        let stack = Box::into_raw(Box::new([0_u8; Self::KERNEL_STACK_SIZE]));
        let stack_bottom = (stack as usize + (unsafe { *stack }).len()) as *mut u8;
        let mut task = Self::new(StackInfo::new(stack as *mut u8, stack_bottom));
        task.thread.context.pc = job as u64;
        task.thread.context.sp = stack_bottom as u64;
        task
    }

    /// Fork a new task from the current task and return the child task pointer
    /// This function should never be inlined because the stack is copied from the parent task to the child task
    #[inline(never)]
    pub fn fork(&self, caller_sp: usize) -> *mut Task {
        // save return address
        let mut return_addr: usize;
        unsafe { asm!("mov {}, lr", out(reg) return_addr) };

        let has_user_stack = self.user_stack.top != core::ptr::null_mut();
        // allocate a new stack for the child task
        let kernel_stack = Box::into_raw(Box::new([0_u8; 1024 * PAGE_SIZE]));
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
            let origin_user_stack =
                unsafe { Box::from_raw(self.user_stack.top as *mut [u8; Self::USER_STACK_SIZE]) };
            let user_stack = Box::into_raw(origin_user_stack.clone());
            Box::into_raw(origin_user_stack);
            let user_stack_bottom = user_stack as usize + (unsafe { *user_stack }).len();
            task.user_stack = StackInfo::new(user_stack as *mut u8, user_stack_bottom as *mut u8);
            task.thread.sp_el0 = SP_EL0.get();
            // workaround for user space page table
            task.memory_mapping
                .map_pages(
                    Self::USER_STACK_START,
                    Some(virt_to_phys(user_stack as usize)),
                    Self::USER_STACK_SIZE,
                    MemoryAttribute::Normal,
                    MemoryAccessPermission::ReadWriteEL1EL0,
                    MemoryExecutePermission::Deny,
                )
                .unwrap();
        };
        // copy the current thread context
        // the registers are stored in the stack in compiler generated function prologue
        unsafe {
            asm!("ldp {}, {}, [x0, -16]
            ldp {}, {}, [x0, -32]
            ldp {}, {}, [x0, -48]
            ldp {}, {}, [x0, -64]
            ldp {}, {}, [x0, -80]
            ldr {}, [x0, -88]", out(reg) task.thread.context.x20, out(reg) task.thread.context.x19, out(reg) task.thread.context.x22, out(reg) task.thread.context.x21, out(reg) task.thread.context.x24, out(reg) task.thread.context.x23, out(reg) task.thread.context.x26, out(reg) task.thread.context.x25, out(reg) task.thread.context.x28, out(reg) task.thread.context.x27, out(reg) task.thread.context.fp, in("x0") caller_sp);
        }
        // child thread will jump to child_entry function and use ret to return to the caller
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
        task.signal_handlers = self.signal_handlers.clone();
        let code = self.user_code.as_ref().unwrap();
        task.user_code = Some(code.clone());
        task.memory_mapping
            .map_pages(
                0,
                Some(virt_to_phys(code.as_ptr() as usize)),
                code.len(),
                MemoryAttribute::Normal,
                MemoryAccessPermission::ReadOnlyEL1EL0,
                MemoryExecutePermission::AllowUser,
            )
            .unwrap();
        task.memory_mapping
            .map_pages(
                GPU_MEMORY_MMIO_BASE,
                Some(phys_to_virt(GPU_MEMORY_MMIO_BASE)),
                GPU_MEMORY_MMIO_SIZE,
                MemoryAttribute::Device,
                MemoryAccessPermission::ReadWriteEL1EL0,
                MemoryExecutePermission::Deny,
            )
            .unwrap();
        let task = Rc::new(Mutex::new(task));
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
    pub fn exit(&mut self, code: usize) -> ! {
        self.state = TaskState::Dead;
        // let the idle task to clean up the task
        // we can't clean up the task here because the task is still running
        scheduler().schedule();
        panic!("Unreachable!")
    }

    pub fn kill(pid: PIDNumber) {
        unsafe { disable_kernel_space_interrupt() }
        if let Some(pid) = pid_manager().get_pid(pid as usize) {
            pid.lock()
                .unwrap()
                .pid_task()
                .unwrap()
                .lock()
                .unwrap()
                .state = TaskState::Dead;
        }
        unsafe { enable_kernel_space_interrupt() }
    }

    #[inline(always)]
    pub fn pid_number(&self) -> PIDNumber {
        self.pid.lock().unwrap().number()
    }

    #[inline(always)]
    pub fn pid(&self) -> Rc<Mutex<PID>> {
        self.pid.clone()
    }

    pub fn run_user_program(&mut self, user_program: &[u8]) {
        let block_len = round_up(user_program.len());
        let mut code_vec = Vec::with_capacity(block_len);
        for byte in user_program {
            code_vec.push(*byte);
        }
        // fill the rest with 0
        // ensure the code is aligned to the page size
        code_vec.resize(block_len, 0);
        let code = code_vec.into_boxed_slice();
        let code_start = code.as_ptr();
        self.user_code = Some(Rc::new(code));
        self.memory_mapping
            .map_pages(
                0,
                Some(virt_to_phys(code_start as usize)),
                block_len,
                MemoryAttribute::Normal,
                MemoryAccessPermission::ReadOnlyEL1EL0,
                MemoryExecutePermission::AllowUser,
            )
            .unwrap();
        self.memory_mapping
            .map_pages(
                GPU_MEMORY_MMIO_BASE,
                Some(phys_to_virt(GPU_MEMORY_MMIO_BASE)),
                GPU_MEMORY_MMIO_SIZE,
                MemoryAttribute::Device,
                MemoryAccessPermission::ReadWriteEL1EL0,
                MemoryExecutePermission::Deny,
            )
            .unwrap();
        let stack_top = Box::into_raw(Box::new([0_u8; Self::USER_STACK_SIZE])) as usize;
        let stack_bottom = stack_top + Self::USER_STACK_SIZE;
        self.memory_mapping
            .map_pages(
                Self::USER_STACK_START,
                Some(virt_to_phys(stack_top)),
                Self::USER_STACK_SIZE,
                MemoryAttribute::Normal,
                MemoryAccessPermission::ReadWriteEL1EL0,
                MemoryExecutePermission::Deny,
            )
            .unwrap();
        self.user_stack.top = stack_top as *mut u8;
        self.user_stack.bottom = stack_bottom as *mut u8;
        unsafe { run_user_code(Self::USER_STACK_END as u64, 0) };
    }

    pub fn set_signal_handler(&mut self, signal: Signal, handler: fn() -> !) {
        self.signal_handlers[signal as usize] = Some(handler);
    }

    pub fn send_signal(&mut self, signal: Signal) {
        self.pending_signals |= 1 << signal as u32;
    }

    pub fn do_pending_signal(&mut self) {
        unsafe { disable_kernel_space_interrupt() }
        if self.doing_signal {
            return;
        }
        self.doing_signal = true;
        for i in 0..Signal::Max as usize {
            if self.pending_signals & (1 << i) != 0 {
                if let Some(handler) = self.signal_handlers[i] {
                    unsafe { enable_kernel_space_interrupt() }
                    // create a new user stack
                    let signal_stack = Box::into_raw(Box::new([0_u8; Self::USER_STACK_SIZE]));
                    let signal_stack_end = signal_stack as u64 + Self::USER_STACK_SIZE as u64;
                    self.signal_stack =
                        StackInfo::new(signal_stack as *mut u8, signal_stack_end as *mut u8);
                    // save the current context
                    let mut context = CPUContext::new();
                    unsafe { disable_kernel_space_interrupt() }
                    unsafe { store_context(&mut context as *mut CPUContext) };
                    // the saved context resume from here
                    self.signal_saved_context = Some((SP_EL0.get(), context));
                    // check if the signal handler has been run
                    // use volatile read to prevent the compiler from optimizing the read
                    if unsafe { core::ptr::read_volatile(&self.pending_signals as *const u32) }
                        & (1 << i)
                        != 0
                    {
                        unsafe {
                            // write the handler to the stack
                            core::ptr::write(
                                (signal_stack_end as usize - (2 * size_of::<fn() -> !>()))
                                    as *mut fn() -> !,
                                handler,
                            );
                            // clear the pending signal
                            self.pending_signals &= !(1 << i);
                            // map the signal handler stack to the user space
                            self.memory_mapping
                                .unmap_pages(Self::USER_STACK_START, Self::USER_STACK_SIZE)
                                .unwrap();
                            self.memory_mapping
                                .map_pages(
                                    Self::USER_STACK_START,
                                    Some(virt_to_phys(signal_stack as usize)),
                                    Self::USER_STACK_SIZE,
                                    MemoryAttribute::Normal,
                                    MemoryAccessPermission::ReadWriteEL1EL0,
                                    MemoryExecutePermission::Deny,
                                )
                                .unwrap();
                            // ensure the signal handler wrapper is accessible in the user space
                            self.memory_mapping
                                .map_pages(
                                    Self::SIGNAL_HANDLER_WRAPPER_SHARE_START,
                                    Some(round_down(virt_to_phys(signal_handler_wrapper as usize))),
                                    PAGE_SIZE,
                                    MemoryAttribute::Normal,
                                    MemoryAccessPermission::ReadOnlyEL1EL0,
                                    MemoryExecutePermission::AllowUser,
                                )
                                .unwrap();
                            enable_kernel_space_interrupt();
                            run_user_code(
                                (Self::USER_STACK_END - (2 * size_of::<fn() -> !>())) as u64,
                                (Self::SIGNAL_HANDLER_WRAPPER_SHARE_START
                                    | (signal_handler_wrapper as usize & 0xfff))
                                    as u64,
                            )
                        };
                    }
                    unsafe { enable_kernel_space_interrupt() }
                } else {
                    unsafe { enable_kernel_space_interrupt() }
                    match Signal::from(i) {
                        Signal::SIGHUP => todo!(),
                        Signal::SIGINT => todo!(),
                        Signal::SIGQUIT => todo!(),
                        Signal::SIGILL => todo!(),
                        Signal::SIGTRAP => todo!(),
                        Signal::SIGABRT => todo!(),
                        Signal::SIGFPE => todo!(),
                        Signal::SIGKILL => signal::default_kill_handler(),
                        Signal::SIGSEGV => todo!(),
                        Signal::SIGPIPE => todo!(),
                        Signal::SIGALRM => todo!(),
                        Signal::SIGTERM => todo!(),
                        _ => unreachable!(),
                    }
                }
            }
        }
        self.doing_signal = false;
    }

    pub fn back_from_signal(&mut self) {
        // release the signal stack
        unsafe {
            let _ = Box::from_raw(self.signal_stack.top as *mut [u8; Task::USER_STACK_SIZE]);
        };
        self.signal_stack = StackInfo::new(core::ptr::null_mut(), core::ptr::null_mut());
        // map the user stack back
        self.memory_mapping
            .unmap_pages(Self::USER_STACK_START, Self::USER_STACK_SIZE)
            .unwrap();
        self.memory_mapping
            .map_pages(
                Self::USER_STACK_START,
                Some(virt_to_phys(self.user_stack.top as usize)),
                Self::USER_STACK_SIZE,
                MemoryAttribute::Normal,
                MemoryAccessPermission::ReadWriteEL1EL0,
                MemoryExecutePermission::Deny,
            )
            .unwrap();
        // invalidate the signal handler wrapper share page
        self.memory_mapping
            .unmap_pages(Self::SIGNAL_HANDLER_WRAPPER_SHARE_START, PAGE_SIZE)
            .unwrap();
        // restore the context
        let (sp, context) = self.signal_saved_context.take().unwrap();
        SP_EL0.set(sp);
        unsafe { load_context(&context as *const CPUContext) };
    }

    #[inline(always)]
    pub fn memory_mapping(&self) -> Rc<MemoryMapping> {
        self.memory_mapping.clone()
    }
}

#[inline(never)]
unsafe fn child_entry() {
    asm!(
        "ldr x0, [sp, -8]
    ldr lr, [sp, -16]
    mov sp, x0
    mov x0, {}", in(reg) current() as usize
    )
}
