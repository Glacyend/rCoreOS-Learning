//! Constants used in rCore for qemu.

pub const MEMORY_END: usize = 0x88000000;

#[allow(unused)]
pub const MMIO: &[(usize, usize)] = &[
    (0x00100000, 0x00002000), // VIRT_TEST/RTC in virt machine
];
