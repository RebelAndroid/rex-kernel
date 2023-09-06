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
            "Attempted to construct PhysicalAddress with address lower than PHYSICAL_MEMORY_SIZE"
        );
        assert!(
            address >= 0x1000,
            "Attempted to construct PhysicalAddress in page 0"
        );
        PhysicalAddress { address }
    }

    /// Gets the address of this `PhysicalAddress`.
    pub fn get_address(&self) -> u64 {
        self.address
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
    pub fn from_physical(physical_address: PhysicalAddress) -> Self{
        DirectMappedAddress { physical_address }
    }

    /// Gets the physical address of this `DirectMappedAddress`.
    pub fn get_physical_address(&self) -> PhysicalAddress{
        self.physical_address
    }

    /// Gets the virtual address of this `DirectMappedAddress`.
    pub fn get_virtual_address(&self) -> u64 {
        self.physical_address.get_address() + DIRECT_MAP_START.get().unwrap()
    }
}