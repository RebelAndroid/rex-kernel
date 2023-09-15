use core::mem::{size_of, align_of};

use crate::{DIRECT_MAP_START, PHYSICAL_MEMORY_SIZE};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalAddress {
    address: u64,
}
impl PhysicalAddress {
    /// Creates a new `PhysicalAddress` with the given address
    pub fn new(address: u64) -> Self {
        assert!(
            address < *PHYSICAL_MEMORY_SIZE.get().unwrap(),
            "Attempted to construct PhysicalAddress with address greater than PHYSICAL_MEMORY_SIZE"
        );
        assert!(
            address >= 0x1000,
            "Attempted to construct PhysicalAddress in page 0, address: {}", address
        );
        PhysicalAddress { address }
    }

    /// Gets the `PhysicalAddress` as a `u64`
    pub fn get_address(&self) -> u64 {
        self.address
    }

    /// Returns whether this physical address is aligned to a 4KB page
    pub fn is_frame_aligned(&self) -> bool {
        // check to see if the bottom 12 bits of the address are clear
        self.address & 0xFFF == 0
    }
}

/// A virtual memory address in the direct physical memory map region of virtual memory
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct DirectMappedAddress {
    physical_address: PhysicalAddress,
}

impl DirectMappedAddress {
    /// Creates a new `DirectMappedAddress` from a virtual address.
    pub fn from_virtual(virtual_address: u64) -> Self {
        assert!(
            virtual_address > *DIRECT_MAP_START.get().unwrap(),
            "Attempted to construct DirectMappedAddress with address lower than DIRECT_MAP_START"
        );
        let physical_address = virtual_address - DIRECT_MAP_START.get().unwrap();
        Self {
            physical_address: PhysicalAddress {
                address: physical_address,
            },
        }
    }

    /// Creates a new `DirectMappedAddress` from a physical address.
    pub fn from_physical(physical_address: PhysicalAddress) -> Self {
        DirectMappedAddress { physical_address }
    }

    /// Gets the physical address of this `DirectMappedAddress`.
    pub fn get_physical_address(&self) -> PhysicalAddress {
        self.physical_address
    }

    /// Gets the virtual address of this `DirectMappedAddress`.
    pub fn get_virtual_address(&self) -> u64 {
        self.physical_address.get_address() + DIRECT_MAP_START.get().unwrap()
    }

    /// Gets a pointer to this direct mapped address.
    pub fn as_pointer<T>(&self) -> *mut T {
        assert!(
            self.physical_address.address + (size_of::<T>() as u64) <= *PHYSICAL_MEMORY_SIZE.get().unwrap(),
            "Attempted to construct pointer to value that exceeds the bounds of physical memory"
        );
        assert_eq!(self.get_virtual_address() % (align_of::<T>() as u64), 0, "Attempted to get unaligned address as pointer!");
        self.get_virtual_address() as *mut T
    }

    /// Gets a pointer to this direct mapped address. This function should be used for structs with sizes not known at compile time (for example, an XSDT).
    pub fn as_pointer_with_size<T>(&self, size: u64) -> *mut T {
        assert!(
            self.physical_address.address + size <= *PHYSICAL_MEMORY_SIZE.get().unwrap(),
            "Attempted to construct pointer to value that exceeds the bounds of physical memory"
        );
        assert_eq!(self.get_virtual_address() % (align_of::<T>() as u64), 0, "Attempted to get unaligned address as pointer!");
        self.get_virtual_address() as *mut T
    }
}