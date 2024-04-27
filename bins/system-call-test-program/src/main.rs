#![no_std]
#![no_main]

mod console;
mod fork_test;
mod print;
mod system_call;

use fork_test::fork_test;
use system_call::{exit, uart_write};

use core::{arch::global_asm, panic::PanicInfo};

global_asm!(include_str!("entry.s"));

#[no_mangle]
pub extern "C" fn _start_rust() {
    // let buf = "test";
    // let buf_start = buf.as_ptr();
    // uart_write(buf_start, buf.len());
    fork_test();
    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
