//! SBI call wrappers.

/// Use SBI call to put char in console (qemu uart handler).
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

/// Use SBI call to get char from console (qemu uart handler).
#[allow(unused)]
pub fn console_getchar() -> usize {
    #[allow(deprecated)]
    sbi_rt::legacy::console_getchar()
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
