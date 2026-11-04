//! Process management syscalls.

use log::info;

use crate::{task, timer};

/// Task exists and submit an exit code.
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {exit_code}");
    task::exit_current_and_run_next();
    unreachable!();
}

/// Current task gives up resources for other tasks.
pub fn sys_yield() -> isize {
    task::suspend_current_and_run_next();
    0
}

/// Get time in milliseconds.
pub fn sys_get_time() -> isize {
    timer::get_time_ms() as isize
}
