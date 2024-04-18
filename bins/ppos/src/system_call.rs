use crate::exception::exception_handler::ExceptionContext;

mod get_pid;

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

impl From<u16> for SystemCallNumber {
    fn from(value: u16) -> Self {
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
        SystemCallNumber::GetPID => get_pid::get_pid(),
        _ => panic!("unsupport system call number"),
    };
    context.set_return_value(result as u64);
}
