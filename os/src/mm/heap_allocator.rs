//! The global allocator.

use core::ptr::addr_of_mut;

use buddy_system_allocator::LockedHeap;

use crate::config::KERNEL_HEAP_SIZE;

/// Heap allocator instance.
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

/// Heap space ([u8; KERNEL_HEAP_SIZE]).
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// Initiate heap allocator.
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
    }
}
