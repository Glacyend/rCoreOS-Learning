//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules.
//! The most important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel.
//! - [`syscall`]: System call handling and implementation.
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`batch::run_next_app()`] and for the first time go to
//! userspace.

#![no_main]
#![no_std]

#[macro_use]
mod console;
pub mod batch;
mod lang_items;
mod logging;
mod sbi;
mod sync;
pub mod syscall;
pub mod trap;

use core::arch::global_asm;

use log::{debug, info, trace};

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

macro_rules! linker_symbol_addr {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

pub(crate) use linker_symbol_addr;

/// The rust entry-point of OS.
#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    unsafe extern "C" {
        /// Start address of text segment.
        safe fn stext();
        /// End address of text segment.
        safe fn etext();
        /// Start address of read-only data segment.
        safe fn srodata();
        /// End address of read-only data segment.
        safe fn erodata();
        /// Start address of data segment.
        safe fn sdata();
        /// End address of data segment.
        safe fn edata();
        /// Start address of BSS segment.
        safe fn sbss();
        /// End address of BSS segment.
        safe fn ebss();
        /// Stack lower bound.
        safe fn boot_stack_lower_bound();
        /// Stack top.
        safe fn boot_stack_top();
    }

    clear_bss();
    logging::init();
    trace!("[kernel] Start running.");

    info!("[kernel] Welcome to the rCoreOS!");

    debug!(
        "[kernel] .text [{:#x}, {:#x})",
        linker_symbol_addr!(stext),
        linker_symbol_addr!(etext)
    );
    debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        linker_symbol_addr!(srodata),
        linker_symbol_addr!(erodata)
    );
    debug!(
        "[kernel] .data [{:#x}, {:#x})",
        linker_symbol_addr!(sdata),
        linker_symbol_addr!(edata)
    );
    debug!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        linker_symbol_addr!(boot_stack_top),
        linker_symbol_addr!(boot_stack_lower_bound)
    );
    debug!(
        "[kernel] .bss [{:#x}, {:#x})",
        linker_symbol_addr!(sbss),
        linker_symbol_addr!(ebss)
    );

    trap::init();
    batch::init();
    batch::run_next_app();
}

/// Clear BSS segment.
fn clear_bss() {
    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    (linker_symbol_addr!(sbss)..linker_symbol_addr!(ebss))
        .for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
