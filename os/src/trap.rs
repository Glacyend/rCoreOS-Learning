//! Trap handling functionality.
//!
//! For rCoreOS, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `_alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, ans syscalls go
//! to [`syscall()`].

use core::arch::global_asm;

use log::info;
use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        scause, stval,
        stvec::{self, TrapMode},
    },
};

pub use crate::trap::context::TrapContext;
use crate::{batch, linker_symbol_addr, syscall};

mod context;

global_asm!(include_str!("trap/trap.S"));

/// Initialize CSR `stvec` at the entry of `__alltraps`.
pub fn init() {
    unsafe extern "C" {
        safe fn __alltraps();
    }
    unsafe {
        stvec::write(stvec::Stvec::new(
            linker_symbol_addr!(__alltraps),
            TrapMode::Direct,
        ));
    }
}

/// Handle an interrupt, exception, or system call from user space.
#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read(); // Get trap cause.
    let stval = stval::read(); // Get extra value.
    let Ok(trap): Result<Trap<Interrupt, Exception>, _> = scause.cause().try_into() else {
        panic!(
            "Unsupported trap {cause:?}, stval = {stval:#x}!",
            cause = scause.cause()
        );
    };
    match trap {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall::syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            info!("[kernel] IllegalInstruction in application, kernel killed it.");
            batch::run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            info!("[kernel] IllegalInstruction in application, kernel killed it.");
            batch::run_next_app();
        }
        _ => {
            panic!("Unsupported trap {trap:?}, stval = {stval:#x}!");
        }
    }

    cx
}
