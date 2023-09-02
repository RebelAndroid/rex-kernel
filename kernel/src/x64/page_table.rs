use bitfield_struct::bitfield;

use crate::pmm::{Frame, FrameAllocator};

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
            .field("Entry", &self.get_entry())
            .finish()
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

    pub fn set_address(&mut self, address: u64) {
        self.set_internal_addr(address >> 12);
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
    /// Used when this page directory entry maps a page table
    page_table: PageDirectoryEntryPageTable,
    /// Used when this page directory entry maps a huge page
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

impl PageDirectory {
    /// Makes a deep copy of this page directory
    fn deep_copy(
        &self,
        frame_allocator: &mut impl FrameAllocator,
        physical_memory_offset: u64,
    ) -> PageDirectory {
        let new_frame = frame_allocator.allocate().unwrap();
        let new_page_directory_ptr =
            (new_frame.get_starting_address() + physical_memory_offset) as *mut PageDirectory;
        // Safe as long as FrameAllocator is implemented correctly
        let mut new_page_directory = unsafe { *new_page_directory_ptr };

        for (i, entry_union) in self.entries.iter().enumerate() {
            if !entry_union.present() {
                // There may be issues with non-present entries because of this
                new_page_directory.entries[i] = *entry_union;
            } else {
                match entry_union.get_entry() {
                    PageDirectoryEntry::PageTable(page_directory_entry_page_table) => {
                        // We need to copy the page table this entry points to
                        let new_page_table_frame = frame_allocator.allocate().unwrap();
                        let new_page_table_ptr = (new_frame.get_starting_address()
                            + physical_memory_offset)
                            as *mut PageTable;
                        unsafe {
                            // Safe as long as FrameAllocator is implemented correctly
                            new_page_table_ptr.write(
                                page_directory_entry_page_table.page_table(physical_memory_offset),
                            )
                        }

                        // The new entry is a copy of the original entry with the address updated to point to the new page table
                        let mut new_entry: PageDirectoryEntryPageTable =
                            page_directory_entry_page_table;
                        // Page tables contain physical addresses
                        new_entry.set_address(new_page_table_frame.get_starting_address());

                        // Add the new entry to the page table
                        new_page_directory.entries[i] = PageDirectoryEntryUnion {
                            page_table: new_entry,
                        };
                    }
                    // For a huge page we can copy over the entry directly
                    PageDirectoryEntry::HugePage(_) => new_page_directory.entries[i] = *entry_union,
                }
            }
        }

        new_page_directory
    }
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

impl Debug for PML4 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(
                self.entries
                    .iter()
                    .enumerate()
                    .filter(|(_, pml4_entry)| pml4_entry.present()),
            )
            .finish()
    }
}
