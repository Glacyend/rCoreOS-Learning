#![no_main]
#![no_std]

#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;

use core::arch::global_asm;
use log::*;

global_asm!(include_str!("entry.asm"));

macro_rules! linker_symbol_addr {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    unsafe extern "C" {
        safe fn stext();
        safe fn etext();
        safe fn srodata();
        safe fn erodata();
        safe fn sdata();
        safe fn edata();
        safe fn sbss();
        safe fn ebss();
        safe fn boot_stack_lower_bound();
        safe fn boot_stack_top();
    }

    clear_bss();
    logging::init();
    trace!("[kernel] Start running.");

    println!("[kernel] Welcome to the rCoreOS!");

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

    info!("[kernel] Kernel running finished.");

    println!("[kernel] rCoreOS shutting down!");
    sbi::shutdown(false);
}

fn clear_bss() {
    unsafe extern "C" {
        safe fn sbss();
        safe fn ebss();
    }
    (linker_symbol_addr!(sbss)..linker_symbol_addr!(ebss))
        .for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
