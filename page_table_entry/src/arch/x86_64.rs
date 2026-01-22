//! x86 page table entries on 64-bit paging.

use core::fmt;

use memory_addr::PhysAddr;

pub use x86_64::structures::paging::page_table::PageTableFlags as PTF;

use crate::{GenericPTE, MappingFlags};

impl From<PTF> for MappingFlags {
    fn from(f: PTF) -> Self {
        if !f.contains(PTF::PRESENT) {
            return Self::empty();
        }
        let mut ret = Self::READ;
        if f.contains(PTF::WRITABLE) {
            ret |= Self::WRITE;
        }
        if !f.contains(PTF::NO_EXECUTE) {
            ret |= Self::EXECUTE;
        }
        if f.contains(PTF::USER_ACCESSIBLE) {
            ret |= Self::USER;
        }
        if f.contains(PTF::NO_CACHE) {
            ret |= Self::UNCACHED;
        }
        ret
    }
}

impl From<MappingFlags> for PTF {
    fn from(f: MappingFlags) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::PRESENT;
        if f.contains(MappingFlags::WRITE) {
            ret |= Self::WRITABLE;
        }
        if !f.contains(MappingFlags::EXECUTE) {
            ret |= Self::NO_EXECUTE;
        }
        if f.contains(MappingFlags::USER) {
            ret |= Self::USER_ACCESSIBLE;
        }
        if f.contains(MappingFlags::DEVICE) || f.contains(MappingFlags::UNCACHED) {
            ret |= Self::NO_CACHE | Self::WRITE_THROUGH;
        }
        ret
    }
}

/// An x86_64 page table entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct X64PTE(u64);

impl X64PTE {
    const PHYS_ADDR_MASK: u64 = 0x000f_ffff_ffff_f000; // bits 12..52
    const SEV_CBIT: u64 = if cfg!(feature = "sev") { 1u64 << 47 } else { 0 };

    fn sev_cbit_for(flags: MappingFlags) -> u64 {
        if cfg!(feature = "sev") && !flags.intersects(MappingFlags::DEVICE | MappingFlags::UNCACHED) {
            Self::SEV_CBIT
        } else {
            0
        }
    }

    /// Creates an empty descriptor with all bits set to zero.
    pub const fn empty() -> Self {
        Self(0)
    }
}

impl GenericPTE for X64PTE {
    fn new_page(paddr: PhysAddr, flags: MappingFlags, is_huge: bool) -> Self {
        let mut flags = PTF::from(flags);
        if is_huge {
            flags |= PTF::HUGE_PAGE;
        }
        let cbit = Self::sev_cbit_for(flags.into());
        Self(flags.bits() | (paddr.as_usize() as u64 & Self::PHYS_ADDR_MASK) | cbit)
    }
    fn new_table(paddr: PhysAddr) -> Self {
        let flags = PTF::PRESENT | PTF::WRITABLE | PTF::USER_ACCESSIBLE;
        Self(
            flags.bits()
                | (paddr.as_usize() as u64 & Self::PHYS_ADDR_MASK)
                | Self::SEV_CBIT,
        )
    }
    fn paddr(&self) -> PhysAddr {
        PhysAddr::from(((self.0 & Self::PHYS_ADDR_MASK) & !Self::SEV_CBIT) as usize)
    }
    fn flags(&self) -> MappingFlags {
        PTF::from_bits_truncate(self.0).into()
    }
    fn set_paddr(&mut self, paddr: PhysAddr) {
        let cbit = self.0 & Self::SEV_CBIT;
        self.0 = (self.0 & !Self::PHYS_ADDR_MASK)
            | (paddr.as_usize() as u64 & Self::PHYS_ADDR_MASK)
            | cbit
    }
    fn set_flags(&mut self, flags: MappingFlags, is_huge: bool) {
        let mut flags = PTF::from(flags);
        if is_huge {
            flags |= PTF::HUGE_PAGE;
        }
        let cbit = Self::sev_cbit_for(flags.into());
        self.0 = (self.0 & Self::PHYS_ADDR_MASK) | flags.bits() | cbit
    }

    fn bits(self) -> usize {
        self.0 as usize
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        PTF::from_bits_truncate(self.0).contains(PTF::PRESENT)
    }
    fn is_huge(&self) -> bool {
        PTF::from_bits_truncate(self.0).contains(PTF::HUGE_PAGE)
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl fmt::Debug for X64PTE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("X64PTE");
        f.field("raw", &self.0)
            .field("paddr", &self.paddr())
            .field("flags", &self.flags())
            .finish()
    }
}
