//! Trap handling functionality.
//!
//! For rCoreOS, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, ans syscalls go
//! to [`syscall::syscall()`].

use core::arch::{asm, global_asm};

use log::warn;
use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        scause, sie, stval,
        stvec::{self, TrapMode},
    },
};

pub use crate::trap::context::TrapContext;
use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    linker_symbol_addr, syscall,
    task::{self, exit_current_and_run_next, suspend_current_and_run_next},
    timer,
};

mod context;

global_asm!(include_str!("trap/trap.S"));

/// Initialize CSR `stvec` at the entry of `__alltraps`.
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(stvec::Stvec::new(
            linker_symbol_addr!(trap_from_kernel),
            TrapMode::Direct,
        ));
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(stvec::Stvec::new(TRAMPOLINE, TrapMode::Direct));
    }
}

/// Enable timer interrupt in `sie` CSR.
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// Handle an interrupt, exception, or system call from user space.
#[unsafe(no_mangle)]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let cx = task::current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();
    let Ok(trap): Result<Trap<Interrupt, Exception>, _> = scause.cause().try_into() else {
        panic!(
            "Unsupported trap {cause:?}, stval = {stval:#X}!",
            cause = scause.cause()
        );
    };
    match trap {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall::syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            warn!(
                "[kernel] PageFault in application, bad addr = {stval:#X}, bad instruction = {sepc:#X}, kernel killed it.",
                sepc = cx.sepc
            );
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            warn!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            timer::set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!("Unsupported trap {trap:?}, stval = {stval:#X}!");
        }
    }

    trap_return();
}

/// Set the new address of `__restore` asm function in [`TRAMPOLINE`] page.
///
/// Set the register `a0` to `trap_cx_ptr`, register `a1` to physical address
/// of user page table.
///
/// Finally, jump to new address of `__restore` asm function.
#[unsafe(no_mangle)]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = task::current_user_token();
    unsafe extern "C" {
        unsafe fn __alltraps();
        unsafe fn __restore();
    }
    let restore_va = linker_symbol_addr!(__restore) - linker_symbol_addr!(__alltraps) + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",  // jump to new addr of `__restore` asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,  // a0 = virt addr of [`TrapContext`]
            in("a1") user_satp,  // a1 = phys addr of user page table
            options(noreturn),
        );
    }
}

/// Unimplement: traps/interrupts/exceptions from kernel mode.
/// Todo: I/O device
#[unsafe(no_mangle)]
pub fn trap_from_kernel() -> ! {
    todo!("a trap from kernel");
}
