//! Implementation of [`MapArea`] and [`MemorySet`].

use core::arch::asm;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use log::trace;
use riscv::register::satp;
use xmas_elf::{ElfFile, program};

use crate::{
    board::MEMORY_END,
    config::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE},
    linker_symbol_addr,
    mm::{
        FrameTracker, PageTableEntry, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum,
        address::{StepByOne, VPNRange},
        frame_alloc,
        page_table::{PTEFlags, PageTable},
    },
    sync::UPSafeCell,
};

unsafe extern "C" {
    safe fn stext();
    safe fn etext();
    safe fn srodata();
    safe fn erodata();
    safe fn sdata();
    safe fn edata();
    safe fn sbss_with_stack();
    safe fn ebss();
    safe fn ekernel();
    safe fn strampoline();
}

lazy_static! {
    /// A memory set instance through [`lazy_static!`] managing kernel space.
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

/// Memory set structure, controls virtual-memory space.
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// # Note
    ///
    /// Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// # Note
    ///
    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(linker_symbol_addr!(strampoline)).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// # Note
    ///
    /// Without kernel stacks.
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        trace!(
            ".text [{:#X}, {:#X})",
            linker_symbol_addr!(stext),
            linker_symbol_addr!(etext)
        );
        trace!(
            ".rodata [{:#X}, {:#X})",
            linker_symbol_addr!(srodata),
            linker_symbol_addr!(erodata)
        );
        trace!(
            ".data [{:#X}, {:#X})",
            linker_symbol_addr!(sdata),
            linker_symbol_addr!(edata)
        );
        trace!(
            ".bss [{:#X}, {:#X})",
            linker_symbol_addr!(sbss_with_stack),
            linker_symbol_addr!(ebss)
        );
        trace!("Mapping .text section.");
        memory_set.push(
            MapArea::new(
                (linker_symbol_addr!(stext)).into(),
                (linker_symbol_addr!(etext)).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        trace!("Mapping .rodata section.");
        memory_set.push(
            MapArea::new(
                (linker_symbol_addr!(srodata)).into(),
                (linker_symbol_addr!(erodata)).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        trace!("Mapping .data section.");
        memory_set.push(
            MapArea::new(
                (linker_symbol_addr!(sdata)).into(),
                (linker_symbol_addr!(edata)).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        trace!("Mapping .bss section.");
        memory_set.push(
            MapArea::new(
                (linker_symbol_addr!(sbss_with_stack)).into(),
                (linker_symbol_addr!(ebss)).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        trace!("Mapping physical memory.");
        memory_set.push(
            MapArea::new(
                (linker_symbol_addr!(ekernel)).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set
    }

    /// Include sections in elf and trampoline and [`TrapContext`] and user
    /// stack, also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();

        // map trampoline
        memory_set.map_trampoline();

        // map program headers of elf, with U flag
        let elf = ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7F, 0x45, 0x4C, 0x46], "invalid elf");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == program::Type::Load {
                let start_va = VirtAddr::from(ph.virtual_addr() as usize);
                let end_va = VirtAddr::from((ph.virtual_addr() + ph.mem_size()) as usize);
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(
                        &elf.input
                            [(ph.offset() as usize)..((ph.offset() + ph.file_size()) as usize)],
                    ),
                );
            }
        }

        // map user stack with U flags
        let max_end_va = VirtAddr::from(max_end_vpn);
        let mut user_stack_bottom: usize = max_end_va.into();

        // guard page
        user_stack_bottom += PAGE_SIZE;

        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // used in sbrk
        memory_set.push(
            MapArea::new(
                user_stack_top.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // map [`TrapContext`]
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp::Satp::from_bits(satp));
            asm!("sfence.vma");
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.shrink_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }

    #[allow(unused)]
    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.append_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
}

/// Map area structure, controls a contigous piece of virtual memory.
pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn = start_va.floor();
        let end_vpn = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
            self.unmap_one(page_table, vpn);
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
            self.map_one(page_table, vpn);
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    /// # Arguments
    ///
    /// - `data`: Start-aligned but maybe with shorter length.
    ///
    /// # Notes
    ///
    /// Assume that all frames were cleared before.
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

/// Map type for [`MemorySet`]: [`Identical`] or [`Framed`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    /// Map permission corresponding to that in pte: `R W X U`.
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}
