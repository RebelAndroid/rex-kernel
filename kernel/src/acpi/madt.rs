use bitflags::bitflags;

use super::root::SDTHeader;

#[repr(packed)]
pub struct MADT {
    header: SDTHeader,
    local_apic_address: u32,
    local_apic_flags: LocalApicFlags,
    entries: *mut u8,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct LocalApicFlags: u32 {
        const LEGACY_PICS = 0x1;
    }
}

#[repr(packed)]
struct MadtEntryHeader {
    entry_type: MadtEntryType,
    entry_length: u8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum MadtEntryType {
    ProcessorLocalApic = 0,
    IOApic = 1,
    IOApicInterruptSourceOverride = 2,
    IOApicNonmaskableInterruptSource = 3,
    LocalApicNonmaskableInterrupts = 4,
    LocalApicAddressOverride = 5,
    ProcessorLocalX2Apic = 9,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessorLocalApic {
    acpi_processor_id: u8,
    apic_id: u8,
    flags: ProcessorLocalApicFlags,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct IOApic {
    apic_id: u8,
    reserved: u8,
    address: u32,
    global_system_interrupt_base: u32,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct IOApicInterruptSourceOverride {
    bus_source: u8,
    irq_source: u8,
    global_system_interrupt: u32,
    flags: IOApicInterruptSourceFlags,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct IOApicNonmaskableInterruptSource {
    non_maskable_interrupt_source: u8,
    reserved: u8,
    flags: IOApicInterruptSourceFlags,
    global_system_interrupt: u32,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct LocalApicNonmaskableInterrupts {
    acpi_processor_id: u8,
    flags: IOApicInterruptSourceFlags,
    lint_number: u8,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct LocalApicAddressOverride {
    reserved: u16,
    physical_address: u64,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessorLocalX2Apic {
    reserved: u16,
    processor_local_x2apic_id: u32,
    flags: LocalApicFlags,
    acpi_id: u32,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ProcessorLocalApicFlags: u32 {
        const PROCESSOR_ENABLED = 0x1;
        const ONLINE_CAPABLE = 0x2;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct IOApicInterruptSourceFlags: u16 {
        const ACTIVE_LOW = 0x10;
        const LEVEL_TRIGGERED = 0x1000;
    }
}

#[derive(Debug)]
pub enum MadtEntry {
    ProcessorLocalApic(ProcessorLocalApic),
    IOApic(IOApic),
    IOApicInterruptSourceOverride(IOApicInterruptSourceOverride),
    IOApicNonmaskableInterruptSource(IOApicNonmaskableInterruptSource),
    LocalApicNonmaskableInterrupts(LocalApicNonmaskableInterrupts),
    LocalApicAddressOverride(LocalApicAddressOverride),
    ProcessorLocalX2Apic(ProcessorLocalX2Apic),
}

struct MadtEntryIterator {
    current: *mut u8,
    max: *mut u8,
}

impl Iterator for MadtEntryIterator {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.max {
            return None;
        }

        let entry_type: MadtEntryType = unsafe { *(self.current as *mut MadtEntryType) };
        let record_length = unsafe { *(self.current.add(1)) };
        let entry_ptr = unsafe { self.current.add(2) };
        Some(match entry_type {
            MadtEntryType::ProcessorLocalApic => {
                MadtEntry::ProcessorLocalApic(unsafe { *(entry_ptr as *mut ProcessorLocalApic) })
            }
            MadtEntryType::IOApic => MadtEntry::IOApic(unsafe { *(entry_ptr as *mut IOApic) }),
            MadtEntryType::IOApicInterruptSourceOverride => {
                MadtEntry::IOApicInterruptSourceOverride(unsafe {
                    *(entry_ptr as *mut IOApicInterruptSourceOverride)
                })
            }
            MadtEntryType::IOApicNonmaskableInterruptSource => {
                MadtEntry::IOApicNonmaskableInterruptSource(unsafe {
                    *(entry_ptr as *mut IOApicNonmaskableInterruptSource)
                })
            }
            MadtEntryType::LocalApicNonmaskableInterrupts => {
                MadtEntry::LocalApicNonmaskableInterrupts(unsafe {
                    *(entry_ptr as *mut LocalApicNonmaskableInterrupts)
                })
            }
            MadtEntryType::LocalApicAddressOverride => {
                MadtEntry::LocalApicAddressOverride(unsafe {
                    *(entry_ptr as *mut LocalApicAddressOverride)
                })
            }
            MadtEntryType::ProcessorLocalX2Apic => {
                MadtEntry::ProcessorLocalX2Apic(unsafe { *(entry_ptr as *mut ProcessorLocalX2Apic) })
            }
        })
    }
}

impl MADT {
    pub fn entries(&self) -> impl Iterator<Item = MadtEntry> {
        MadtEntryIterator {
            current: self.entries,
            max: unsafe { self.entries.add(self.header.length as usize - 0x28) },
        }
    }
}
