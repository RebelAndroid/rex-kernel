use bitflags::bitflags;

use super::root::SDTHeader;
#[repr(packed)]
pub struct MADT {
    header: SDTHeader,
    local_apic_address: u32,
    local_apic_flags: LocalApicFlags,
}

bitflags!{
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
enum MadtEntryType {
    ProcessorLocalApic = 0,
    IOApic = 1,
    IOApicInterruptSourceOverride = 2,
    IOApicNonmaskableInterruptSource = 3,
    LocalApicNonmaskableInterrupts = 4,
    LocalApicAddressOverride = 5,
    ProcessorLocalX2Apic = 9,
}

struct ProcessorLocalApicEntry {
    acpi_processor_id: u8,
    apic_id: u8,
    flags: ProcessorLocalApicFlags,
}

bitflags!{
    pub struct ProcessorLocalApicFlags: u32 {
        const PROCESSOR_ENABLED = 0x1;
        const ONLINE_CAPABLE = 0x2;
    }
}