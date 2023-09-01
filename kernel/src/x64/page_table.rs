use bitfield_struct::bitfield;

use crate::pmm::Frame;

use core::fmt::Debug;

/// An entry in PML4 that references a page directory pointer table
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
    /// The address bits of the entry, **do not use directly**, use `address()`.
    #[bits(40)]
    internal_addr: u64,
    #[bits(11)]
    __: u16,
    execute_disable: bool,
}

impl Pml4Entry {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    pub fn pdpte(&self, physical_memory_offset: u64) -> Pdpte {
        let ptr = (self.address() + physical_memory_offset) as *mut Pdpte;
        unsafe { *ptr }
    }
}

/// An entry in a page directory pointer table that references a page directory
#[bitfield(u64)]
pub struct PdpteEntryPageDirectory {
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

impl PdpteEntryPageDirectory {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    pub fn page_directory(&self, physical_memory_offset: u64) -> PageDirectory {
        let ptr = (self.address() + physical_memory_offset) as *mut PageDirectory;
        unsafe { *ptr }
    }
}

/// An entry in a page directory pointer table that references a 1GB Page
#[bitfield(u64)]
pub struct PdpteEntryHugePage {
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

impl PdpteEntryHugePage {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 30
    }

    pub fn frame(&self) -> ! {
        todo!("huge pages not implemented")
    }
}

#[derive(Clone, Copy)]
pub union PdpteEntryUnion {
    page_directory: PdpteEntryPageDirectory,
    huge_page: PdpteEntryHugePage,
}

impl PdpteEntryUnion {
    /// Gets the appropriate type of entry
    pub fn get_entry(&self) -> PdpteEntry {
        if unsafe { self.huge_page.page_size() } {
            // This is safe because any entry where the page_size bit is set represents a huge page
            PdpteEntry::HugePage(unsafe { self.huge_page })
        } else {
            // This is safe because any entry where the page_size bit is clear represents a page directory
            PdpteEntry::PageDirectory(unsafe { self.page_directory })
        }
    }
}

impl Debug for PdpteEntryUnion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PdpteEntryUnion")
            .field("Entry", &self.get_entry()).finish()
    }
}

#[derive(Debug)]
pub enum PdpteEntry {
    PageDirectory(PdpteEntryPageDirectory),
    HugePage(PdpteEntryHugePage),
}

/// An entry in a page directory that references a page table
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

impl PageDirectoryEntryPageTable {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    pub fn page_table(&self, physical_memory_offset: u64) -> PageTable {
        let ptr = (self.address() + physical_memory_offset) as *mut PageTable;
        unsafe { *ptr }
    }
}

/// An entry in a page directory that references a 2MB page
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

impl PageDirectoryEntryHugePage {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 21
    }

    pub fn frame(&self) -> ! {
        todo!("huge frames not implemented")
    }
}

#[derive(Clone, Copy)]
pub union PageDirectoryEntryUnion {
    page_directory: PageDirectoryEntryPageTable,
    huge_page: PageDirectoryEntryHugePage,
}

impl PageDirectoryEntryUnion {
    /// Gets the appropriate type of entry
    pub fn get_entry(&self) -> PageDirectoryEntry {
        if unsafe { self.huge_page.page_size() } {
            // This is safe because any entry where the page_size bit is set represents a huge page
            PageDirectoryEntry::HugePage(unsafe { self.huge_page })
        } else {
            // This is safe because any entry where the page_size bit is clear represents a page directory
            PageDirectoryEntry::PageTable(unsafe { self.page_directory })
        }
    }
}

impl Debug for PageDirectoryEntryUnion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageDirectoryEntryUnion")
            .field("Entry", &self.get_entry()).finish()
    }
}

#[derive(Debug)]
pub enum PageDirectoryEntry {
    PageTable(PageDirectoryEntryPageTable),
    HugePage(PageDirectoryEntryHugePage),
}

/// An entry in a page table that maps a 4KB page
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

impl PageTableEntry {
    /// Gets the address pointed to by this page table entry.
    fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    /// Gets the frame mapped by this page table entry.
    fn frame(&self) -> Frame {
        Frame::from_starting_address(self.address())
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct PageDirectory {
    pub entries: [PageDirectoryEntryUnion; 512],
}

#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct Pdpte {
    pub entries: [PdpteEntryUnion; 512],
}

#[derive(Clone, Copy)]
pub struct PML4 {
    pub entries: [Pml4Entry; 512],
}
