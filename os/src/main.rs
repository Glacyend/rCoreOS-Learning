//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules.
//! The most important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel.
//! - [`task`]: Task management.
//! - [`syscall`]: System call handling and implementation.
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_first_task()`] and for the first time go to
//! userspace.

#![no_main]
#![no_std]

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
mod logging;
mod sbi;
mod sync;
pub mod syscall;
pub mod task;
pub mod trap;

use core::arch::global_asm;

use indoc::indoc;
use log::{info, trace};

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
    clear_bss();
    logging::init();
    trace!("[kernel] Start running.");

    let log = indoc! {r#"
        .______        ______   ______   .______       _______   ______        _______.
        |   _  \      /      | /  __  \  |   _  \     |   ____| /  __  \      /       |
        |  |_)  |    |  ,----'|  |  |  | |  |_)  |    |  |__   |  |  |  |    |   (----`
        |      /     |  |     |  |  |  | |      /     |   __|  |  |  |  |     \   \
        |  |\  \----.|  `----.|  `--'  | |  |\  \----.|  |____ |  `--'  | .----)   |
        | _| `._____| \______| \______/  | _| `._____||_______| \______/  |_______/
    "#};
    println!("{}", log);

    info!("[kernel] Welcome to rCoreOS!");

    trap::init();
    loader::load_apps();
    task::run_first_task();

    unreachable!()
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
