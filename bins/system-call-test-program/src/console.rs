use core::fmt;

use crate::system_call::system_call;

pub struct Console {}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe { system_call(2, s.as_ptr() as usize, s.len(), 0, 0, 0, 0) };
        Ok(())
    }
}

static mut CONSOLE: Console = Console {};

pub fn console() -> &'static mut Console {
    unsafe { &mut CONSOLE }
}
