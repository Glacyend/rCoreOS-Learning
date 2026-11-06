//! Implementation of [`TaskContext`].

use crate::{linker_symbol_addr, trap};

/// Task context structure containing some registers.
#[repr(C)]
pub struct TaskContext {
    /// Return address (e.g. __restore) of __switch ASM function.
    ra: usize,
    /// Kernel stack pointer of app.
    sp: usize,
    /// Callee saved registers:  s 0..11
    s: [usize; 12],
}

impl TaskContext {
    /// Init task context.
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// Set [`TaskContext`].
    ///
    /// ```text
    /// TaskContext {
    ///     __restore ASM function: trap_return,
    ///     sp: kstack_ptr,
    ///     s: s_0..12,
    /// }
    /// ```
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: linker_symbol_addr!(trap::trap_return),
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
