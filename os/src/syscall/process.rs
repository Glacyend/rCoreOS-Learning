//! App management syscalls.

use log::info;

use crate::batch;

/// Task exists and submit an exit code.
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {exit_code}");
    batch::run_next_app();
}
