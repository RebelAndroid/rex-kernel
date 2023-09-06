use bitfield_struct::bitfield;

use crate::{pmm::{Frame, FrameAllocator}, DEBUG_SERIAL_PORT};

use core::fmt::{Debug, Write};

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
    /// The address bits of the entry, **do not use directly**, use `address()` and `set_address()`.
    #[bits(40)]
    internal_addr: u64,
    #[bits(11)]
    __: u16,
    execute_disable: bool,
}

impl Pml4Entry {
    /// Returns the address associated with this Pml4Entry
    pub fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    /// Sets the address associated with this Pml4Entry
    pub fn set_address(&mut self, address: u64) {
        assert_eq!(
            address,
            address & (!0xFFF),
            "bottom 12 bits of address should be zero"
        );
        self.set_internal_addr(address >> 12)
    }

    /// Returns the Pdpte referenced by this Pml4Entry
    pub fn pdpt(&self, physical_memory_offset: u64) -> Pdpt {
        let ptr = (self.address() + physical_memory_offset) as *mut Pdpt;
        unsafe { *ptr }
    }
}

/// An entry in a page directory pointer table that references a page directory
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

impl PdptEntryPageDirectory {
    /// Gets the physical address associated with this Pdpt entry
    pub fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    pub fn set_address(&mut self, address: u64) {
        assert_eq!(
            address,
            address & (!0xFFF),
            "bottom 12 bits of address should be zero"
        );
        self.set_internal_addr(address >> 12);
    }

    /// Gets the page directory associated with this Pdpt entry, assuming physical memory is mapped at physical_memory_offset
    pub fn page_directory(&self, physical_memory_offset: u64) -> PageDirectory {
        let ptr = (self.address() + physical_memory_offset) as *mut PageDirectory;
        unsafe { *ptr }
    }
}

/// An entry in a page directory pointer table that references a 1GB Page
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

impl PdptEntryHugePage {
    pub fn address(&self) -> u64 {
        self.internal_addr() << 30
    }

    pub fn frame(&self) -> ! {
        todo!("huge pages not implemented")
    }
}

#[derive(Clone, Copy)]
pub union PdptEntryUnion {
    page_directory: PdptEntryPageDirectory,
    huge_page: PdptEntryHugePage,
}

impl PdptEntryUnion {
    /// Gets the appropriate type of entry
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

#[derive(Debug)]
pub enum PdptEntry {
    PageDirectory(PdptEntryPageDirectory),
    HugePage(PdptEntryHugePage),
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
    /// Gets the physical address of the PageTable referenced by this PageDirectoryEntry
    pub fn address(&self) -> u64 {
        self.internal_addr() << 12
    }

    pub fn set_address(&mut self, address: u64) {
        assert_eq!(
            address,
            address & (!0xFFF),
            "bottom 12 bits of address should be zero"
        );
        self.set_internal_addr(address >> 12);
    }

    pub fn page_table(&self, physical_memory_offset: u64) -> PageTable {
        writeln!(DEBUG_SERIAL_PORT.lock(), "address: {:x}", self.address());
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
    /// Makes a deep copy of this page directory, returning it and its physical address
    fn deep_copy(
        &self,
        frame_allocator: &mut impl FrameAllocator,
        physical_memory_offset: u64,
    ) -> (PageDirectory, u64) {
        let new_frame = frame_allocator.allocate().unwrap();
        let new_page_directory_ptr =
            (new_frame.get_starting_address() + physical_memory_offset) as *mut PageDirectory;
        // Safe as long as FrameAllocator is implemented correctly
        let mut new_page_directory = unsafe { *new_page_directory_ptr };

        for (i, entry_union) in self.entries.iter().enumerate() {
            if !entry_union.present() {
                // There may be issues with non-present entries because of this if they contain data that is cannot be directly copied
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

        (new_page_directory, new_frame.get_starting_address())
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct Pdpt {
    pub entries: [PdptEntryUnion; 512],
}

impl Pdpt {
    /// Makes a deep copy of this Pdpt. Returns the copy and its physical address.
    fn deep_copy(
        &self,
        frame_allocator: &mut impl FrameAllocator,
        physical_memory_offset: u64,
    ) -> (Pdpt, u64) {
        // Create new Pdpt
        let new_frame: Frame = frame_allocator.allocate().unwrap();
        writeln!(DEBUG_SERIAL_PORT.lock(), "pdpt frame: {:x?}", new_frame);
        let new_pdpt_ptr: *mut Pdpt =
            (new_frame.get_starting_address() + physical_memory_offset) as *mut Pdpt;
        // Safe as long as FrameAllocator is implemented correctly
        let mut new_pdpt: Pdpt = unsafe { *new_pdpt_ptr };

        for (i, entry_union) in self.entries.iter().enumerate() {
            if !entry_union.present() {
                new_pdpt.entries[i] = *entry_union;
            } else {
                match entry_union.get_entry() {
                    PdptEntry::PageDirectory(entry_to_page_directory) => {
                        let (new_page_directory, new_page_directory_addr) = entry_to_page_directory
                            .page_directory(physical_memory_offset)
                            .deep_copy(frame_allocator, physical_memory_offset);
                        let mut new_entry: PdptEntryPageDirectory = entry_to_page_directory.clone();
                        new_entry.set_address(new_page_directory_addr);
                    }
                    // For a huge page we can copy over the entry directly
                    PdptEntry::HugePage(_) => new_pdpt.entries[i] = *entry_union,
                }
            }
        }

        (new_pdpt, new_frame.get_starting_address())
    }
}

#[derive(Clone, Copy)]
pub struct PML4 {
    pub entries: [Pml4Entry; 512],
}

impl PML4 {
    /// Makes a deep copy of this PML4. Returns the copy and its physical address
    pub fn deep_copy(
        &self,
        frame_allocator: &mut impl FrameAllocator,
        physical_memory_offset: u64,
    ) -> (PML4, u64) {
        let new_frame: Frame = frame_allocator.allocate().unwrap();
        writeln!(DEBUG_SERIAL_PORT.lock(), "new PML4 Frame: {:x?}", new_frame);
        let new_pml4_ptr: *mut PML4 =
            (new_frame.get_starting_address() + physical_memory_offset) as *mut PML4;
        let mut new_pml4: &mut PML4 = unsafe {&mut *new_pml4_ptr };

        // Copy all of the entries of the PML4
        for (i, entry) in self.entries.iter().enumerate() {
            writeln!(DEBUG_SERIAL_PORT.lock(), "deep copying entry {}", i);
            let pdpt = entry.pdpt(physical_memory_offset);
            let (new_pdpt, new_pdpt_address) =
                pdpt.deep_copy(frame_allocator, physical_memory_offset);
            let mut new_entry = entry.clone();
            new_entry.set_address(new_pdpt_address);
            new_pml4.entries[i] = new_entry
        }

        (*new_pml4, new_frame.get_starting_address())
    }
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
