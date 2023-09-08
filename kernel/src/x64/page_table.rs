use bitfield_struct::bitfield;

use core::fmt::{Debug, Write};

use crate::{
    memory::{DirectMappedAddress, PhysicalAddress},
    pmm::{Frame, FrameAllocator},
    FRAME_ALLOCATOR, DEBUG_SERIAL_PORT,
};
/// The top level paging structure, each entry references a Pdpt
#[derive(Clone, Copy)]
pub struct PML4 {
    pub entries: [Pml4Entry; 512],
}

/// Mid level paging structure, each entry references a page directory or maps a 1GB page.
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct Pdpt {
    pub entries: [PdptEntryUnion; 512],
}

/// Mid level paging structure, each entry references a page table or maps a 2MB page.
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct PageDirectory {
    pub entries: [PageDirectoryEntryUnion; 512],
}

/// Bottom level paging structure, each entry maps a 4KB page.
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

/// An entry in PML4 that references a page directory pointer table.
#[bitfield(u64)]
pub struct Pml4Entry {
    present: bool,
    read_write: bool,
    user_supervisor: bool,
    page_write_through: bool,
    page_cache_disable: bool,
    accessed: bool,
    __: bool,
    __: bool,
    #[bits(3)]
    __: u8,
    /// Only used in HLAT paging.
    restart: bool,
    /// The address bits of the entry, **do not use directly**, use `address()` and `set_address()`.
    #[bits(40)]
    internal_addr: u64,
    #[bits(11)]
    __: u16,
    execute_disable: bool,
}

/// An entry in a page directory pointer table. It either references a page directory or maps a 1GB page; this is represented by the two union members.
/// `PdptEntry` is provided as a safe wrapper.
#[derive(Clone, Copy)]
pub union PdptEntryUnion {
    page_directory: PdptEntryPageDirectory,
    huge_page: PdptEntryHugePage,
}

/// An entry in a page directory pointer table that references a 1GB Page.
#[bitfield(u64)]
pub struct PdptEntryHugePage {
    present: bool,
    read_write: bool,
    user_supervisor: bool,
    page_write_through: bool,
    page_cache_disable: bool,
    accessed: bool,
    dirty: bool,
    page_size: bool,
    global: bool,
    #[bits(2)]
    __: u8,
    /// Only used in HLAT paging.
    restart: bool,
    page_attribute_table: bool,
    #[bits(17)]
    __: u32,
    /// The address bits of the entry, **do not use directly**, use `address()`.
    #[bits(22)]
    internal_addr: u64,
    __: bool,
    #[bits(7)]
    __: u8,
    #[bits(3)]
    protection_key: u8,
    execute_disable: bool,
}

/// An entry in a page directory pointer table that references a page directory.
#[bitfield(u64)]
pub struct PdptEntryPageDirectory {
    present: bool,
    read_write: bool,
    user_supervisor: bool,
    page_write_through: bool,
    page_cache_disable: bool,
    accessed: bool,
    __: bool,
    page_size: bool,
    #[bits(3)]
    __: u8,
    /// Only used in HLAT paging.
    restart: bool,
    /// The address bits of the entry, **do not use directly**, use `address()`.
    #[bits(40)]
    internal_addr: u64,
    #[bits(11)]
    __: u16,
    execute_disable: bool,
}

/// A safe wrapper for `PdptEntryUnion`.
#[derive(Debug)]
pub enum PdptEntry {
    PageDirectory(PdptEntryPageDirectory),
    HugePage(PdptEntryHugePage),
}

/// An entry in a page directory. It either references a page table or maps a 2MB page; this is represented by the two union members.
/// `PageDirectoryEntry` is provided as a safe wrapper.
#[derive(Clone, Copy)]
pub union PageDirectoryEntryUnion {
    /// Used when this page directory entry maps a page table
    page_table: PageDirectoryEntryPageTable,
    /// Used when this page directory entry maps a huge page
    huge_page: PageDirectoryEntryHugePage,
}

/// An entry in a page directory that references a 2MB page.
#[bitfield(u64)]
pub struct PageDirectoryEntryHugePage {
    present: bool,
    read_write: bool,
    user_supervisor: bool,
    page_write_through: bool,
    page_cache_disable: bool,
    accessed: bool,
    dirty: bool,
    page_size: bool,
    global: bool,
    #[bits(2)]
    __: u8,
    /// Only used in HLAT paging.
    restart: bool,
    page_attribute_table: bool,
    #[bits(8)]
    __: u8,
    /// The address bits of the entry, **do not use directly**, use `address()`.
    #[bits(31)]
    internal_addr: u64,
    #[bits(7)]
    __: u8,
    #[bits(4)]
    __: u8,
    execute_disable: bool,
}

/// An entry in a page directory that references a page table.
#[bitfield(u64)]
pub struct PageDirectoryEntryPageTable {
    present: bool,            // 0
    read_write: bool,         // 1
    user_supervisor: bool,    // 2
    page_write_through: bool, // 3
    page_cache_disable: bool, // 4
    accessed: bool,           // 5
    __: bool,                 // 6
    page_size: bool,          // 7
    #[bits(3)]
    __: u8,     // 10:8
    /// Only used in HLAT paging.
    restart: bool, // 11
    /// The address bits of the entry, **do not use directly**, use `address()`.
    #[bits(40)]
    internal_addr: u64, // 51:12
    #[bits(11)]
    __: u16,   // 62:52
    execute_disable: bool,    // 63
}

/// A safe wrapper for `PageDirectoryEntryUnion`.
#[derive(Debug)]
pub enum PageDirectoryEntry {
    PageTable(PageDirectoryEntryPageTable),
    HugePage(PageDirectoryEntryHugePage),
}

/// An entry in a page table that maps a 4KB page.
#[bitfield(u64)]
pub struct PageTableEntry {
    present: bool,
    read_write: bool,
    user_supervisor: bool,
    page_write_through: bool,
    page_cache_disable: bool,
    accessed: bool,
    dirty: bool,
    page_attribute_table: bool,
    global: bool,
    #[bits(2)]
    __: u8,
    /// Only used in HLAT paging.
    restart: bool,
    /// The address bits of the entry, **do not use directly**, use `address()`.
    #[bits(40)]
    internal_addr: u64,
    __: bool,
    #[bits(7)]
    __: u8,
    #[bits(3)]
    protection_key: u8,
    execute_disable: bool,
}

// Implement the basic operations of a Pml4Entry
impl Pml4Entry {
    /// Returns the address associated with this Pml4Entry.
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.internal_addr() << 12)
    }

    /// Sets the address associated with this Pml4Entry.
    pub fn set_address(&mut self, physical_address: PhysicalAddress) {
        self.set_internal_addr(physical_address.get_address() >> 12);
    }

    /// Returns the Pdpte referenced by this Pml4Entry.
    pub fn pdpt(&self) -> *mut Pdpt {
        (DirectMappedAddress::from_physical(PhysicalAddress::from(self.address())))
            .as_pointer::<Pdpt>()
    }

    pub fn set_pdpt(&mut self, page_directory_pointer_table: *const Pdpt) {
        let direct_mapped_address =
            DirectMappedAddress::from_virtual(page_directory_pointer_table as u64);
        self.set_address(direct_mapped_address.get_physical_address())
    }
}

// Implement the basic operations of a PdptEntryUnion
impl PdptEntryUnion {
    /// Converts this union to its safe wrapper: `PdptEntry`
    pub fn get_entry(&self) -> PdptEntry {
        if unsafe { self.huge_page.page_size() } {
            // This is safe because any entry where the page_size bit is set represents a huge page
            PdptEntry::HugePage(unsafe { self.huge_page })
        } else {
            // This is safe because any entry where the page_size bit is clear represents a page directory
            PdptEntry::PageDirectory(unsafe { self.page_directory })
        }
    }

    /// Returns whether the present bit is set in this entry
    pub fn present(&self) -> bool {
        // This is safe because it doesn't matter if we use huge_page or page_table, the present bit is the same
        unsafe { self.huge_page.present() }
    }
}

impl Debug for PdptEntryUnion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PdpteEntryUnion")
            .field("Entry", &self.get_entry())
            .finish()
    }
}

// Implement the basic operations of a PageDirectoryEntryUnion
impl PageDirectoryEntryUnion {
    /// Gets the appropriate type of entry
    pub fn get_entry(&self) -> PageDirectoryEntry {
        if unsafe { self.huge_page.page_size() } {
            // This is safe because any entry where the page_size bit is set represents a huge page
            PageDirectoryEntry::HugePage(unsafe { self.huge_page })
        } else {
            // This is safe because any entry where the page_size bit is clear represents a page directory
            PageDirectoryEntry::PageTable(unsafe { self.page_table })
        }
    }

    /// Checks whether this entry is present
    pub fn present(&self) -> bool {
        // This is safe because it doesn't matter if we use huge_page or page_table, the present bit is the same
        unsafe { self.huge_page.present() }
    }
}

impl Debug for PageDirectoryEntryUnion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageDirectoryEntryUnion")
            .field("Entry", &self.get_entry())
            .finish()
    }
}

// Implement the basic operations of a PdptEntryPageDirectory
impl PdptEntryPageDirectory {
    /// Gets the physical address pointed to by this entry
    fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.internal_addr() << 12)
    }

    /// Sets the physical address pointed to by this entry
    fn set_address(&mut self, physical_address: PhysicalAddress) {
        assert!(physical_address.is_frame_aligned());
        self.set_internal_addr(physical_address.get_address() >> 12);
    }

    /// Gets the page directory associated with this Pdpt entry.
    pub fn page_directory(&self) -> *mut PageDirectory {
        DirectMappedAddress::from_physical(self.address()).as_pointer::<PageDirectory>()
    }

    /// Sets this page directory pointer table entry address to point to the given page directory.
    /// Requires that page_directory is located in direct mapped memory
    pub fn set_page_directory(&mut self, page_directory: *const PageDirectory) {
        let direct_mapped_address = DirectMappedAddress::from_virtual(page_directory as u64);
        self.set_address(direct_mapped_address.get_physical_address())
    }
}

// Implement the basic operations of a PdptEntryHugePage
impl PdptEntryHugePage {
    /// Gets the physical address referenced by this Pdpt entry
    fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.internal_addr() << 30)
    }

    pub fn frame(&self) -> ! {
        todo!("huge pages not implemented")
    }
}

impl PageDirectoryEntryPageTable {
    /// Gets the physical address of the page table referenced by this page directory entry
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.internal_addr() << 12)
    }

    pub fn set_address(&mut self, physical_address: PhysicalAddress) {
        assert!(physical_address.is_frame_aligned());
        self.set_internal_addr(physical_address.get_address() >> 12);
    }

    /// Gets the page table referenced by this page directory entry.
    pub fn page_table(&self) -> *mut PageTable {
        DirectMappedAddress::from_physical(self.address()).as_pointer::<PageTable>()
    }

    /// Sets this page directory entry address to point to the given page table.
    /// Requires that page_table is located in direct mapped memory
    pub fn set_page_table(&mut self, page_table: *const PageTable) {
        let direct_mapped_address = DirectMappedAddress::from_virtual(page_table as u64);
        self.set_address(direct_mapped_address.get_physical_address())
    }
}

impl PageDirectoryEntryHugePage {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 21
    }

    pub fn frame(&self) -> ! {
        todo!("huge frames not implemented")
    }
}

/// Implement the basic operations of a `PageTableEntry`
impl PageTableEntry {
    /// Gets the address pointed to by this page table entry.
    fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.internal_addr() << 12)
    }

    /// Gets the frame mapped by this page table entry.
    fn frame(&self) -> Frame {
        Frame::from_starting_address(self.address())
    }
}

impl PML4 {
    /// Makes a copy of this page directory pointer table, allocating a new frame using the static `FRAME_ALLOCATOR`.
    pub fn copy(&self) -> Self {
        let new = {
            let frame = FRAME_ALLOCATOR.get().unwrap().lock().allocate().unwrap();
            let pointer = DirectMappedAddress::from_physical(frame.get_starting_address())
                .as_pointer::<PML4>();
            writeln!(DEBUG_SERIAL_PORT.lock(), "creating new PML4 at {:x?}", frame.get_starting_address());
            unsafe { &mut *pointer }
        };

        for (i, entry) in self.entries.iter().enumerate() {
            writeln!(DEBUG_SERIAL_PORT.lock(), "creating new entry: {}", i);
            let mut new_entry = entry.clone();
            let deep_copy_pdpt = unsafe{*(entry.pdpt())}.copy();
            new_entry.set_pdpt(&deep_copy_pdpt);
            new.entries[i] = new_entry;
        }
        *new
    }
}

impl Pdpt {
    /// Makes a copy of this page directory pointer table, allocating a new frame using the static `FRAME_ALLOCATOR`.
    pub fn copy(&self) -> Self {
        let new = {
            let frame = FRAME_ALLOCATOR.get().unwrap().lock().allocate().unwrap();
            let pointer = DirectMappedAddress::from_physical(frame.get_starting_address())
                .as_pointer::<Pdpt>();
            writeln!(DEBUG_SERIAL_PORT.lock(), "creating new Pdpt at {:x?}", frame.get_starting_address());
            unsafe { &mut *pointer }
        };
        for (i, entry_union) in self.entries.iter().enumerate() {
            new.entries[i] = if !entry_union.present() {
                *entry_union
            } else {
                match entry_union.get_entry() {
                    // An entry that maps a huge page can be directly copied.
                    PdptEntry::HugePage(_) => *entry_union,
                    PdptEntry::PageDirectory(entry) => {
                        let mut new_entry: PdptEntryPageDirectory = entry;
                        let deep_copy_page_directory = unsafe { *entry.page_directory() }.copy();
                        new_entry.set_page_directory(&deep_copy_page_directory);
                        PdptEntryUnion {
                            page_directory: new_entry,
                        }
                    }
                }
            };
        }
        *new
    }
}

impl PageDirectory {
    /// Makes a copy of this page directory, allocating a new frame using the static `FRAME_ALLOCATOR`.
    pub fn copy(&self) -> Self {
        let new = {
            let frame = FRAME_ALLOCATOR.get().unwrap().lock().allocate().unwrap();
            let pointer = DirectMappedAddress::from_physical(frame.get_starting_address())
                .as_pointer::<PageDirectory>();
            writeln!(DEBUG_SERIAL_PORT.lock(), "creating new PageDirectory at {:x?}", frame.get_starting_address());
            unsafe { &mut *pointer }
        };
        for (i, entry_union) in self.entries.iter().enumerate() {
            new.entries[i] = if !entry_union.present() {
                *entry_union
            } else {
                match entry_union.get_entry() {
                    // An entry that maps a huge page can be directly copied.
                    PageDirectoryEntry::HugePage(_) => *entry_union,
                    PageDirectoryEntry::PageTable(entry) => {
                        let mut new_entry: PageDirectoryEntryPageTable = entry;
                        let deep_copy_page_table = unsafe { *entry.page_table() }.copy();
                        new_entry.set_page_table(&deep_copy_page_table);
                        PageDirectoryEntryUnion {
                            page_table: new_entry,
                        }
                    }
                }
            };
        }
        *new
    }
}

impl PageTable {
    /// Makes a copy of this page table, allocating a new frame using the static `FRAME_ALLOCATOR`.
    pub fn copy(&self) -> Self {
        let frame = FRAME_ALLOCATOR.get().unwrap().lock().allocate().unwrap();
        let pointer = DirectMappedAddress::from_physical(frame.get_starting_address())
            .as_pointer::<PageTable>();
        writeln!(DEBUG_SERIAL_PORT.lock(), "creating new PageTable at {:x?}", frame.get_starting_address());
        unsafe {
            pointer.write(*self);
            writeln!(DEBUG_SERIAL_PORT.lock(), "finished creating new PageTable");
            pointer.read()
        }
        
    }
}
