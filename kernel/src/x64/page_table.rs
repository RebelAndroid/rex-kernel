use bitfield_struct::bitfield;

use core::fmt::{Debug, Write};

use crate::{
    memory::{DirectMappedAddress, PhysicalAddress, VirtualAddress},
    pmm::{Frame, FrameAllocator, MemoryMapAllocator},
    DEBUG_SERIAL_PORT, FRAME_ALLOCATOR,
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

struct PageTableIterator<'a> {
    page_table: &'a PML4,
    current: VirtualAddress,
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

    /// Makes this `Pml4Entry` point to the given `Pdpt`. The pointer should be in direct mapped memory
    pub fn set_pdpt(&mut self, page_directory_pointer_table: *const Pdpt) {
        let direct_mapped_address = DirectMappedAddress::from_virtual(VirtualAddress::create(
            page_directory_pointer_table as u64,
        ));
        self.set_address(direct_mapped_address.get_physical_address())
    }
}

// Implement the basic operations of a PdptEntryUnion
impl PdptEntryUnion {
    pub fn new(x: u64) -> Self {
        Self {
            huge_page: PdptEntryHugePage::from(x),
        }
    }

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
    pub fn new(x: u64) -> Self {
        Self {
            huge_page: PageDirectoryEntryHugePage::from(x),
        }
    }

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

    /// Returns true if this PageDirectoryEntry maps a huge page.
    pub fn huge_page(&self) -> bool {
        unsafe { self.huge_page }.page_size()
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
        let direct_mapped_address =
            DirectMappedAddress::from_virtual(VirtualAddress::create(page_directory as u64));
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
        let direct_mapped_address =
            DirectMappedAddress::from_virtual(VirtualAddress::create(page_table as u64));
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

    /// Sets the address pointed to by this page table entry
    fn set_address(&mut self, physical_address: PhysicalAddress) {
        assert!(
            physical_address.is_frame_aligned(),
            "Attempted to map page to non-frame-aligned physical address"
        );
        self.set_internal_addr(physical_address.get_address() >> 12);
    }

    /// Causes this frame to map the given frame.
    fn set_frame(&mut self, frame: Frame) {
        self.set_address(frame.get_starting_address());
    }
}

impl PML4 {
    /// Creates a new empty pml4 table
    pub fn new() -> &'static mut Self {
        let physical_address = FRAME_ALLOCATOR
            .get()
            .unwrap()
            .lock()
            .allocate()
            .unwrap()
            .get_starting_address();
        let direct_address = DirectMappedAddress::from_physical(physical_address);
        let mut pml4 = unsafe { direct_address.as_pointer::<Self>().as_mut().unwrap() };
        for i in 0..512 {
            pml4.entries[i] = Pml4Entry::from(0u64);
        }
        pml4
    }

    /// Maps `virtual_address` to `frame`
    pub fn map(
        &mut self,
        frame: Frame,
        virtual_address: VirtualAddress,
        writable: bool,
        no_execute: bool,
    ) {
        let mut pml4_entry = self.entries[virtual_address.pml4_index()];
        let pdpt = if pml4_entry.present() {
            unsafe { pml4_entry.pdpt().as_mut().unwrap() }
        } else {
            // create a new pdpt
            let new_pdpt = Pdpt::new();
            // and add it to this pml4
            pml4_entry.set_pdpt(new_pdpt as *const Pdpt);
            pml4_entry.set_present(true);

            new_pdpt
        };

        let pdpt_entry = pdpt.entries[virtual_address.pdpt_index()];
        let page_directory = if pdpt_entry.present() {
            match pdpt_entry.get_entry() {
                PdptEntry::PageDirectory(page_directory_pointer) => unsafe {
                    page_directory_pointer.page_directory().as_mut().unwrap()
                },
                PdptEntry::HugePage(_) => panic!("Tried to map already mapped page!"),
            }
        } else {
            PageDirectory::new()
        };
        let page_directory_entry = page_directory.entries[virtual_address.page_directory_index()];
        let page_table = if page_directory_entry.present() {
            match page_directory_entry.get_entry() {
                PageDirectoryEntry::PageTable(page_table_pointer) => unsafe {
                    page_table_pointer.page_table().as_mut().unwrap()
                },
                PageDirectoryEntry::HugePage(_) => panic!("Tried to map already mapped page!"),
            }
        } else {
            PageTable::new()
        };
        let mut page_table_entry: PageTableEntry =
            page_table.entries[virtual_address.page_table_index()];
        assert!(
            !page_table_entry.present(),
            "tried to map already mapped page"
        );
        page_table_entry.set_frame(frame);
        page_table_entry.set_read_write(writable);
        page_table_entry.set_execute_disable(no_execute);
    }

    /// Gets an iterator over the mappings of this PML4's page table hierarchy
    pub fn iterator(&self) -> PageTableIterator {
        PageTableIterator {
            page_table: self,
            current: VirtualAddress::create(0),
        }
    }
}

impl Pdpt {
    /// Creates a new empty pdpt.
    pub fn new() -> &'static mut Self {
        let physical_address = FRAME_ALLOCATOR
            .get()
            .unwrap()
            .lock()
            .allocate()
            .unwrap()
            .get_starting_address();
        let direct_address = DirectMappedAddress::from_physical(physical_address);
        let mut pdpt = unsafe { direct_address.as_pointer::<Self>().as_mut().unwrap() };
        for i in 0..512 {
            pdpt.entries[i] = PdptEntryUnion::new(0u64);
        }
        pdpt
    }
}

impl PageDirectory {
    /// Creates a new empty page directory.
    pub fn new() -> &'static mut Self {
        let physical_address = FRAME_ALLOCATOR
            .get()
            .unwrap()
            .lock()
            .allocate()
            .unwrap()
            .get_starting_address();
        let direct_address = DirectMappedAddress::from_physical(physical_address);
        let mut page_directory = unsafe { direct_address.as_pointer::<Self>().as_mut().unwrap() };
        for i in 0..512 {
            page_directory.entries[i] = PageDirectoryEntryUnion::new(0u64);
        }
        page_directory
    }
}

impl PageTable {
    /// Creates a new empty page table
    pub fn new() -> &'static mut Self {
        let physical_address = FRAME_ALLOCATOR
            .get()
            .unwrap()
            .lock()
            .allocate()
            .unwrap()
            .get_starting_address();
        let direct_address = DirectMappedAddress::from_physical(physical_address);
        let mut page_table = unsafe { direct_address.as_pointer::<Self>().as_mut().unwrap() };
        for i in 0..512 {
            page_table.entries[i] = PageTableEntry::from(0u64);
        }
        page_table
    }
}

impl Iterator for PageTableIterator<'_>{
    type Item = (VirtualAddress, Frame);

    fn next(&mut self) -> Option<Self::Item> {
        let x: u64 = self.current.into();
        let (new, overflow) = x.overflowing_add(1 << 12); // Go to next page.
        if overflow {
            return None;
        }
        self.current = VirtualAddress::create(new);
        if !self.page_table.entries[self.current.pml4_index()].present() {
            // If the PML4 entry is not present, we can jump to the next one,
            // If we are at the last one, we can finish by returning None
            if self.current.pml4_index() == 1 << 9 {
                return None
            }
            self.current.set_pml4_index(self.current.pml4_index() + 1)
            self.current.set_pdpt_index(0);
            self.current.set_page_directory_index(0);
            self.current.set_page_table_index(0);
            return self.next();
        }
        let pdpt = unsafe{self.page_table.entries[self.current.pml4_index()].pdpt().as_ref()}.unwrap();
        if !self.page_table.entries[self.current.pdpt_index()].present() {
            if self.current.pdpt_index() == 1 << 9 {
                if self.current.pml4_index() == 1 << 9 {
                    return None
                }
                self.current.set_pml4_index(self.current.pml4_index() + 1);
                self.current.set_pdpt_index(0);
                self.current.set_page_directory_index(0);
                self.current.set_page_table_index(0);
            }
            self.current.set_pdpt_index(self.current.pdpt_index() + 1);
            self.current.set_page_directory_index(0);
            self.current.set_page_table_index(0);
            return self.next();
        }
        let page_directory = pdpt.entries[self.current.pdpt_index()];
        // Todo, handle huge pages.
    }
}