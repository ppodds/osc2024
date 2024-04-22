use crate::exception::exception_handler::ExceptionContext;

mod exec;
mod get_pid;
mod uart_read;
mod uart_write;

pub enum SystemCallNumber {
    GetPID,
    UARTRead,
    UARTWrite,
    Exec,
    Fork,
    Exit,
    MBoxCall,
    Kill,
}

impl From<u64> for SystemCallNumber {
    fn from(value: u64) -> Self {
        match value {
            0 => SystemCallNumber::GetPID,
            1 => SystemCallNumber::UARTRead,
            2 => SystemCallNumber::UARTWrite,
            3 => SystemCallNumber::Exec,
            4 => SystemCallNumber::Fork,
            5 => SystemCallNumber::Exit,
            6 => SystemCallNumber::MBoxCall,
            7 => SystemCallNumber::Kill,
            _ => panic!("unsupport system call number"),
        }
    }
}

pub fn system_call(
    context: &mut ExceptionContext,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) {
    let result = match context.system_call_number().into() {
        SystemCallNumber::GetPID => get_pid::get_pid() as usize,
        SystemCallNumber::UARTRead => uart_read::uart_read(arg0 as *mut char, arg1),
        SystemCallNumber::UARTWrite => uart_write::uart_write(arg0 as *const char, arg1),
        SystemCallNumber::Exec => {
            exec::exec(arg0 as *const char, arg1 as *const *const char) as usize
        }
        _ => panic!("unsupport system call number"),
    };
    context.set_return_value(result as u64);
}
