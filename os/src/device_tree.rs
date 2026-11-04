//! Device tree helpers.

use core::sync::atomic::{AtomicUsize, Ordering};

use fdt::Fdt;
use log::info;

static CLOCK_FREQ: AtomicUsize = AtomicUsize::new(0);

/// Initializes early device information from the flattened device tree.
pub(crate) fn init(dtb_pa: usize) {
    let fdt = unsafe { Fdt::from_ptr(dtb_pa as *const u8) }.expect("invalid device tree");
    let clock_freq = fdt
        .cpus()
        .next()
        .map(|cpu| cpu.timebase_frequency())
        .expect("missing timebase-frequency in device tree");
    CLOCK_FREQ.store(clock_freq, Ordering::Relaxed);
    info!("CLOCK_FREQ from device tree: {clock_freq}");
}

/// Returns the number of `mtime` ticks per second.
pub(crate) fn clock_freq() -> usize {
    let clock_freq = CLOCK_FREQ.load(Ordering::Relaxed);
    assert!(clock_freq != 0, "device tree is not initialized");
    clock_freq
}
