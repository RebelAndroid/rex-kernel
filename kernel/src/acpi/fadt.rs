use core::mem::{offset_of, size_of};

use super::root::SDTHeader;

#[repr(C)]
#[derive(Debug)]
pub struct FADT {
    header: SDTHeader,
    /// The physical address of the FACS. If `x_firmware_control` is non-zero, it should be ignored.
    firmware_control: u32,
    /// The physical address of the DSDT. If `x_dsdt` is non-zero, it should be ignored.
    dsdt: u32,
    reserved: u8,
    /// The preferred power management profile of the device. TODO: change to enum.
    preferred_power_management_profile: u8,
    sci_interrupt: u16,
    smi_command_port: u32,
    acpi_enable: u8,
    acpi_disable: u8,
    s4bios_req: u8,
    pstate_control: u8,
    pm1a_event_block: u32,
    pm1b_event_block: u32,
    pm1a_control_block: u32,
    pm1b_control_block: u32,
    pm2_control_block: u32,
    pm_timer_block: u32,
    gpe0_block: u32,
    gpe1_block: u32,
    pm1_event_length: u8,
    pm1_control_length: u8,
    pm2_control_length: u8,
    pm_timer_length: u8,
    gpe0_length: u8,
    gpe1_length: u8,
    gpe1_base: u8,
    cstate_control: u8,
    worst_c2_latency: u16,
    worst_c3_latency: u16,
    flush_size: u16,
    flush_stride: u16,
    duty_offset: u8,
    duty_width: u8,
    day_alarm: u8,
    month_alarm: u8,
    century: u8,
    boot_architecture_flags: u16,
    reserved2: u8,
    flags: u32,
    reset_register: GenericAddressStructure,
    reset_value: u8,
    reserved3: [u8 ; 3],
    x_firmware_control: u64,
    x_dsdt: u64,
    x_pm1a_event_block: GenericAddressStructure,
    x_pm1b_event_block: GenericAddressStructure,
    x_pm1a_control_block: GenericAddressStructure,
    x_pm1b_control_block: GenericAddressStructure,
    x_pm2_control_block: GenericAddressStructure,
    x_pm_timer_block: GenericAddressStructure,
    x_gpe0_block: GenericAddressStructure,
    x_gpe1_block: GenericAddressStructure,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum AddressSpace {
    SystemMemory = 0,
    SystemIO = 1,
    PciConfigurationSpace = 2,
    EmbeddedController = 3,
    SystemManagementBus = 4,
    SystemCmos = 5,
    PciDeviceBarTarget = 6,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum AccessSize {
    ByteAccess = 1,
    TwoByteAccess = 2,
    FourByteAccess = 3,
    EightByteAccess = 4,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct GenericAddressStructure {
    address_space: AddressSpace,
    bit_width: u8,
    bit_offset: u8,
    access_size: AccessSize,
    address: u64,
}

impl GenericAddressStructure {
    pub fn check_offsets() {
        assert_eq!(offset_of!(GenericAddressStructure, address_space), 0);
        assert_eq!(offset_of!(GenericAddressStructure, bit_width), 1);
        assert_eq!(offset_of!(GenericAddressStructure, bit_offset), 2);
        assert_eq!(offset_of!(GenericAddressStructure, access_size), 3);
        assert_eq!(offset_of!(GenericAddressStructure, address), 4);
        assert_eq!(size_of::<GenericAddressStructure>(), 12);
    }
}

impl FADT{
    /// Checks the offsets of FADT fields. Panics if any are incorrect
    pub fn check_offsets() {

        assert_eq!(offset_of!(FADT, header), 0);
        assert_eq!(offset_of!(FADT, firmware_control), 36);
        assert_eq!(offset_of!(FADT, dsdt), 40);
        assert_eq!(offset_of!(FADT, preferred_power_management_profile), 45);
        assert_eq!(offset_of!(FADT, sci_interrupt), 46);
        assert_eq!(offset_of!(FADT, smi_command_port), 48);
        assert_eq!(offset_of!(FADT, acpi_enable), 52);
        assert_eq!(offset_of!(FADT, acpi_disable), 53);
        assert_eq!(offset_of!(FADT, s4bios_req), 54);
        assert_eq!(offset_of!(FADT, pstate_control), 55);
        assert_eq!(offset_of!(FADT, pm1a_event_block), 56);
        assert_eq!(offset_of!(FADT, pm1b_event_block), 60);
        assert_eq!(offset_of!(FADT, pm1a_control_block), 64);
        assert_eq!(offset_of!(FADT, pm1b_control_block), 68);
        assert_eq!(offset_of!(FADT, pm2_control_block), 72);
        assert_eq!(offset_of!(FADT, pm_timer_block), 76);
        assert_eq!(offset_of!(FADT, gpe0_block), 80);
        assert_eq!(offset_of!(FADT, gpe1_block), 84);
        assert_eq!(offset_of!(FADT, pm1_event_length), 88);
        assert_eq!(offset_of!(FADT, pm1_control_length), 89);
        assert_eq!(offset_of!(FADT, pm2_control_length), 90);

        assert_eq!(offset_of!(FADT, x_gpe0_block), 232);
    }
}