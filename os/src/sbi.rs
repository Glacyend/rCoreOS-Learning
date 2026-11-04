//! SBI call wrappers.

/// Use SBI call to put char in console (qemu uart handler).
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

/// Use SBI call to set timer.
pub fn set_timer(timer: usize) {
    sbi_rt::set_timer(timer as u64);
}

/// Use SBI call to shutdown the kernel.
pub fn shutdown(failure: bool) -> ! {
    if !failure {
        sbi_rt::system_reset(sbi_rt::Shutdown, sbi_rt::NoReason);
    } else {
        sbi_rt::system_reset(sbi_rt::Shutdown, sbi_rt::SystemFailure);
    }
    unreachable!();
}
