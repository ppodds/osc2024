use core::slice;

use cpu::cpu::enable_kernel_space_interrupt;
use library::console::console;

pub fn uart_read(buf: *mut u8, size: usize) -> usize {
    let mut buffer = unsafe { slice::from_raw_parts_mut(buf, size) };
    unsafe { enable_kernel_space_interrupt() };
    for i in 0..size {
        loop {
            if let Some(c) = console().read_char() {
                buffer[i] = c as u8;
                break;
            }
        }
    }
    size
}
