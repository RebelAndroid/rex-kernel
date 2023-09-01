use core::arch::asm;

use super::{gdt::SegmentSelector, page_table::PML4};

use bitflags::bitflags;

/// Reads the value of the cs register.
pub fn get_cs() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, cs", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the ds register.
pub fn get_ds() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, ds", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the ss register.
pub fn get_ss() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, ss", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the es register.
pub fn get_es() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, es", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the fs register.
pub fn get_fs() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, fs", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the gs register.
pub fn get_gs() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, gs", output = out(reg) x) }
    SegmentSelector { x }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Cr0: u64{
        /// Enables protection. If paging is not set, only enables segment level protection.
        const protection_enable = 1;
        const monitor_coprocessor = 1 << 1;
        const emulation = 1 << 2;
        const task_switched = 1 << 3;
        const extension_type = 1 << 4;
        const numeric_error = 1 << 5;
        const write_protect = 1 << 16;
        const alignment_mask = 1 << 18;
        const not_write_through = 1 << 29;
        const cache_disable = 1 << 30;
        /// Enables paging, requires protection_enable to be set.
        const paging = 1 << 31;
    }
}

/// Reads the value of the cr0 register.
pub fn get_cr0() -> Cr0 {
    let x: u64;
    unsafe { asm!("mov {}, cr0", out(reg) x) }
    Cr0::from_bits_retain(x)
}

#[repr(transparent)]
pub struct Cr3 {
    x: u64,
}

impl Cr3 {
    pub fn new(x: u64) -> Self {
        Cr3 { x }
    }

    /// Gets the physical address described by this cr3 value
    pub fn address(&self) -> u64 {
        const M: u64 = 52;
        // clear the bottom 12 bits of cr3, and the top bits above M
        self.x & (!0xFFF) & (!(u64::MAX << M))
    }

    /// Gets the PML4 pointed to by cr3 (requires physical memory to be mapped at some offset)
    pub fn pml4(&self, physical_memory_offset: u64) -> PML4 {
        let ptr = (self.address() + physical_memory_offset) as *mut PML4;
        unsafe { *ptr }
    }
}

/// Reads the value of the CR3 register.
pub fn get_cr3() -> Cr3 {
    let x: u64;
    unsafe { asm!("mov {c}, cr3", c = out(reg) x) }
    Cr3::new(x)
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Cr4: u64{
        const virtual_8086_extensions = 1;
        const protected_mode_virtual_interrupts = 1 << 1;
        const time_stamp_disable = 1 << 2;
        const debugging_extensions = 1 << 3;
        const page_size_extensions = 1 << 4;
        const physical_address_extension = 1 << 5;
        const machine_check_enable = 1 << 6;
        const page_global_enable = 1 << 7;
        const performance_counter_enable = 1 << 8;
        const os_fxsavestore_enable = 1 << 9;
        const os_unmasked_simd_floating_point_exceptions = 1 << 10;
        const user_mode_instruction_prevention = 1 << 11;
        const five_level_paging = 1 << 12;
        const vmx_enable = 1 << 13;
        const smx_enable = 1 << 14;
        const fsgsbase_enable = 1 << 16;
        const pcid_enable = 1 << 17;
        const xsave_processor_extended_states_enable = 1 << 18;
        const key_locker_enable = 1 << 19;
        const smep_enable = 1 << 20;
        const smap_enable = 1 << 21;
        const protection_keys_for_user_pages_enable = 1 << 22;
        const control_flow_enforcement = 1 << 23;
        const protection_keys_for_supervisor_pages_enable = 1 << 24;
        const user_interrupts_enable = 1 << 25;
    }
}

/// Reads the value of the CR3 register.
pub fn get_cr4() -> Cr4 {
    let x: u64;
    unsafe { asm!("mov {c}, cr4", c = out(reg) x) }
    Cr4::from_bits_retain(x)
}
