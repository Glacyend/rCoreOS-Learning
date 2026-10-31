//! Implementation of [`TaskContext`].

use crate::linker_symbol_addr;

/// Task Context.
#[derive(Clone, Copy)]
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

    /// Set task context {__restore ASM function, kernel stack, s_0..12}.
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        unsafe extern "C" {
            unsafe fn __restore();
        }
        Self {
            ra: linker_symbol_addr!(__restore),
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
