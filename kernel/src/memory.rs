use core::mem::{align_of, size_of};

use bitfield_struct::bitfield;

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
            "Attempted to construct PhysicalAddress in page 0, address: {}",
            address
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
    pub fn from_virtual(virtual_address: VirtualAddress) -> Self {
        assert!(
            virtual_address.address() > *DIRECT_MAP_START.get().unwrap(),
            "Attempted to construct DirectMappedAddress with address lower than DIRECT_MAP_START"
        );
        let physical_address = virtual_address.address() - DIRECT_MAP_START.get().unwrap();
        assert!(physical_address < *PHYSICAL_MEMORY_SIZE.get().unwrap());
        Self {
            physical_address: PhysicalAddress::new(virtual_address.address()),
        }
    }

    pub fn try_from_virtual(virtual_address: VirtualAddress) -> Option<Self> {
        if virtual_address.address() <= *DIRECT_MAP_START.get().unwrap() {
            return None;
        }

        let physical_address = virtual_address.address() - DIRECT_MAP_START.get().unwrap();
        if physical_address >= *PHYSICAL_MEMORY_SIZE.get().unwrap() {
            return None;
        }
        Some(Self {
            physical_address: PhysicalAddress {
                address: physical_address,
            },
        })
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
    pub fn get_virtual_address(&self) -> VirtualAddress {
        VirtualAddress::create(self.physical_address.get_address() + DIRECT_MAP_START.get().unwrap())
    }

    /// Gets a pointer to this direct mapped address.
    pub fn as_pointer<T>(&self) -> *mut T {
        assert!(
            self.physical_address.address + (size_of::<T>() as u64)
                <= *PHYSICAL_MEMORY_SIZE.get().unwrap(),
            "Attempted to construct pointer to value that exceeds the bounds of physical memory"
        );
        assert_eq!(
            self.get_virtual_address().address() % (align_of::<T>() as u64),
            0,
            "Attempted to get unaligned address as pointer!"
        );
        self.get_virtual_address().address() as *mut T
    }

    /// Gets a pointer to this direct mapped address. This function should be used for structs with sizes not known at compile time (for example, an XSDT).
    pub fn as_pointer_with_size<T>(&self, size: u64) -> *mut T {
        assert!(
            self.physical_address.address + size <= *PHYSICAL_MEMORY_SIZE.get().unwrap(),
            "Attempted to construct pointer to value that exceeds the bounds of physical memory"
        );
        assert_eq!(
            self.get_virtual_address().address() % (align_of::<T>() as u64),
            0,
            "Attempted to get unaligned address as pointer!"
        );
        self.get_virtual_address().address() as *mut T
    }
}

/// A 48-bit virtual address
/// # Do not create with `new`, use `create` to ensure validity
#[bitfield(u64)]
pub struct VirtualAddress {
    #[bits(12)]
    page_offset: usize,
    #[bits(9)]
    page_table_index: usize,
    #[bits(9)]
    page_directory_index: usize,
    #[bits(9)]
    pdpt_index: usize,
    #[bits(9)]
    pml4_index: usize,
    sign_extension: u16,
}

impl VirtualAddress {
    /// Creates a new virtual address
    /// Panics if `virtual_address` is non canonical
    pub fn create(virtual_address: u64) -> Self {
        let new = Self::from(virtual_address);

        assert!(
            (new.sign_extension() == 0 && new.pml4_index() & 1 << 8 == 0)
                || (new.sign_extension() == 0xFFFF && new.pml4_index() & 1 << 8 == 1 << 8),
            "Attempted to create non canonical virtual address {:x}, {:x?}", virtual_address, new
        );

        new
    }

    pub fn address(&self) -> u64 {
        (*self).into()
    }
}
