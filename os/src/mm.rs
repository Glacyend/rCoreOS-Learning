//! Memory management implementation.
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like [`FrameAllocator`], [`PageTable`],
//! [`MapArea`] and [`MemorySet`], is implemented here.
//!
//! Every task or process has a `memory_set` to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use frame_allocator::{FrameTracker, frame_alloc};
pub use memory_set::{KERNEL_SPACE, MapPermission, MemorySet};
pub use page_table::{PageTableEntry, translated_byte_buffer};

/// Initiate [`heap_allocator`], [`frame_allocator`] and [`KERNEL_SPACE`].
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
