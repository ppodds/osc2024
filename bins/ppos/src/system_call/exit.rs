use crate::scheduler::task::Task;

pub fn exit() {
    Task::exit(0);
}
