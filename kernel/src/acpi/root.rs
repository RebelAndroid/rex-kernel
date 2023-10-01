use core::mem::{size_of};

use core::fmt::Write;

use crate::{acpi_signature, DEBUG_SERIAL_PORT};
use crate::memory::{DirectMappedAddress, PhysicalAddress};


use super::fadt::FADT;
use super::madt::MADT;

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
#[derive(Debug, Clone, Copy)]
pub struct SDTHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

#[repr(packed)]
#[derive(Debug)]
pub struct XSDT {
    header: SDTHeader,
    pub sdts: PhysicalAddress,
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
        unsafe { validate_checksum(self as *const _ as *const u8, self.header.length as usize) }
    }

    /// Gets the number of entries in the table.
    pub fn length(&self) -> u64 {
        // divide by 8 because all entries are 8 byte pointers
        (self.header.length as u64 - size_of::<SDTHeader>() as u64) / 8
    }

    /// Gets the `index`-th pointer in the table.
    /// Panics if index is out of range
    pub fn get_pointer(&self, index: u64) -> *mut SDTHeader {
        assert!(index < self.length(), "index out of bounds in XSDT");
        // Assertion makes this safe
        let header_address = unsafe {
            let array_base = (self as *const _ as *const PhysicalAddress).byte_offset(36);
            let header_pointer = array_base.add(index as usize);
            header_pointer.read_unaligned()
        };
        DirectMappedAddress::from_physical(header_address).as_pointer::<SDTHeader>()
    }

    /// Gets the table with the given signature
    pub fn get_table(&self, signature: [u8; 4]) -> Option<*mut SDTHeader> {
        for i in 0..self.length() {
            
            let x = unsafe { &mut *self.get_pointer(i) };
            if x.signature == signature {
                return Some(self.get_pointer(i));
            }
        }
        None
    }

    /// Gets the Multiple APIC Descriptor Table associated with this XSDT.
    pub fn get_madt(&self) -> Option<&mut MADT> {
        let ptr = self.get_table(acpi_signature!('A', 'P', 'I', 'C'))? as *mut MADT;
        let madt = unsafe {ptr.as_mut()};
        if let Some(ref i) = madt{
            assert!(i.checksum(), "Found MADT that did not pass checksum!");
        }
        madt
    }

    /// Gets the Fixed ACPI Description Table associated with this XSDT.
    pub fn get_fadt(&self) -> Option<&mut FADT> {
        let ptr = self.get_table(acpi_signature!('F', 'A', 'C', 'P'))? as *mut FADT;
        unsafe {ptr.as_mut()}
    }
}

/// Returns whether `size` bytes starting at `start` sum to 0.
/// Used to validate ACPI tables.
/// Safe if the range of addresses starting at start and of length `size` is valid.
pub unsafe fn validate_checksum(start: *const u8, size: usize) -> bool {
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