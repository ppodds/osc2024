use core::slice;

use library::console::console;

pub fn uart_write(buf: *const char, size: usize) -> usize {
    let buffer = unsafe { slice::from_raw_parts(buf, size) };
    for c in buffer {
        console().write_char(*c);
    }
    size
}
