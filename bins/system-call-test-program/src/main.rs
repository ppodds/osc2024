#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};

global_asm!(include_str!("entry.s"));

extern "C" {
    fn system_call(
        number: u64,
        arg0: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
    ) -> usize;
}

#[no_mangle]
pub extern "C" fn _start_rust() {
    let buf = "test";
    let buf_start = buf.as_ptr();
    unsafe { system_call(2, buf_start as usize, buf.len(), 0, 0, 0, 0) };
    unsafe { system_call(5, 0, 0, 0, 0, 0, 0) };
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
