//! RISC-V timer-related functionality.

use riscv::register::time;

use crate::{device_tree, sbi};

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

/// Read the `mtime` register.
pub fn get_time() -> usize {
    time::read()
}

/// Get current time in microseconds.
pub fn get_time_ms() -> usize {
    time::read() / (device_tree::clock_freq() / MSEC_PER_SEC)
}

/// Set the next timer interrupt.
pub fn set_next_trigger() {
    sbi::set_timer(get_time() + device_tree::clock_freq() / TICKS_PER_SEC);
}
