use crate::{acpi_signature, DEBUG_SERIAL_PORT};
use crate::memory::{DirectMappedAddress, PhysicalAddress};
use core::fmt::Write;

#[repr(packed)]
#[derive(Debug)]
pub struct RSDP32Bit {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(packed)]
#[derive(Debug)]
pub struct RSDP64Bit {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    deprecated: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(packed)]
#[derive(Debug)]
pub struct SDTHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

#[repr(packed)]
pub struct XSDT {
    header: SDTHeader,
    SDTs: *mut (),
}

impl RSDP32Bit {
    /// Returns whether this RSDP satisfies the checksum
    pub fn checksum(&self) -> bool {
        let mut sum: u8 = 0;
        for b in self.signature {
            sum = sum.wrapping_add(b);
        }
        sum = sum.wrapping_add(self.checksum);
        for b in self.oem_id {
            sum = sum.wrapping_add(b);
        }
        sum = sum.wrapping_add(self.revision);
        for b in self.rsdt_address.to_ne_bytes() {
            sum = sum.wrapping_add(b);
        }
        sum == 0
    }

    /// Gets the ACPI revision of the RSDP
    pub fn revision(&self) -> u8 {
        self.revision
    }
}

impl RSDP64Bit {
    pub fn checksum(&self) -> bool {
        let mut sum: u8 = 0;
        for b in self.signature {
            sum = sum.wrapping_add(b);
        }
        sum = sum.wrapping_add(self.checksum);
        for b in self.oem_id {
            sum = sum.wrapping_add(b);
        }
        sum = sum.wrapping_add(self.revision);
        for b in self.deprecated.to_ne_bytes() {
            sum = sum.wrapping_add(b);
        }
        for b in self.length.to_ne_bytes() {
            sum = sum.wrapping_add(b);
        }
        for b in self.xsdt_address.to_ne_bytes() {
            sum = sum.wrapping_add(b);
        }
        sum = sum.wrapping_add(self.extended_checksum);
        for b in self.reserved {
            sum = sum.wrapping_add(b);
        }
        sum == 0
    }

    pub fn get_xsdt(&self) -> *mut XSDT {
        let ptr = DirectMappedAddress::from_physical(PhysicalAddress::new(self.xsdt_address))
            .as_pointer::<XSDT>();
        let size = unsafe { ptr.read() }.header.length as u64;
        assert!(unsafe { validate_checksum(ptr as *const u8, size as usize) });
        DirectMappedAddress::from_physical(PhysicalAddress::new(self.xsdt_address))
            .as_pointer_with_size::<XSDT>(size)
    }
}

impl XSDT {
    /// Validates the checksum and signature of this XSDT, returning true if they are both valid.
    pub fn checksum(&self) -> bool {
        if self.header.signature != acpi_signature!('X', 'S', 'D', 'T') {
            return false;
        }
        // This is safe because an XSDT can only be constructed from `RSDP64Bit::get_xsdt()` which checks that the entire table is in memory
        writeln!(DEBUG_SERIAL_PORT.lock(), "starting address: {:x}", self as *const _ as *const u8 as u64);
        unsafe { validate_checksum(self as *const _ as *const u8, self.header.length as usize) }
    }
}

/// Returns whether `size` bytes starting at `start` sum to 0.
/// Used to validate ACPI tables.
/// Safe if the range of addresses starting at start and of length `size` is valid.
unsafe fn validate_checksum(start: *const u8, size: usize) -> bool {
    writeln!(DEBUG_SERIAL_PORT.lock(), "validating from {:x} with length {:x}", start as u64, size);
    let mut sum: u8 = 0;
    for i in 0..size {
        let byte = start.add(i).read();
        //write!(DEBUG_SERIAL_PORT.lock(), "byte: {:x} ", byte);
        sum = sum.wrapping_add(byte);
    }
    sum == 0
}

#[macro_export]
macro_rules! acpi_signature {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        [$a as u8, $b as u8, $c as u8, $d as u8]
    };
}
