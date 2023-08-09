use bitflags::bitflags;
use core::arch::asm;

#[derive(Debug)]
#[repr(packed)]
pub struct GDTR {
    pub size: u16,
    pub base: u64,
}

impl GDTR {
    /// loads this GDTR
    /// caller must ensure that self is a valid GDTR
    pub unsafe fn load(&self) {
        asm!("lgdt [{gdtr}]", gdtr = in(reg) &self);
    }

    pub fn get() -> Self {
        let x: *mut GDTR;

        unsafe {
            asm!("sgdt [{gdtr}]", gdtr = out(reg) x);
            x.read()
        }
    }

    /// Gets the segment descriptor at the specified index (or none if the index is out of range)
    /// caller must ensure that self is a valid GDTR
    pub unsafe fn get_segment_descriptor(&self, index: usize) -> Option<&SegmentDescriptor> {
        // the size of the table in bytes is size + 1, divide by the size of a Segment Descriptor
        // (8 bytes) to get the number of segment descriptors in the GDT
        let table_entries = (self.size as usize + 1) / 8;
        if index >= table_entries {
            return None;
        } else {
            let ptr = self.base as *mut SegmentDescriptor;
            return unsafe { ptr.add(index).as_ref() };
        }
    }

    /// Creates a GDTR that covers the provided array of segment descriptors
    /// May panic if segment_descriptors is too large (the length of the array in bytes - 1 overflows a u16)
    pub fn from_segment_descriptors(&self, segment_descriptors: &[SegmentDescriptor]) -> GDTR {
        let size = (segment_descriptors.len() * 8) - 1.try_into().unwrap();
        let base = segment_descriptors.as_ptr() as u64;

        GDTR { size, base }
    }
}

#[derive(Debug)]
#[repr(packed)]
pub struct SegmentDescriptor {
    limit: u16,
    base1: u16,
    base2: u8,
    pub access_byte: AccessByte,
    limit2_and_flags: u8,
    base3: u8,
}

impl SegmentDescriptor {
    pub fn get_limit(&self) -> u32 {
        let mut limit: u32 = self.limit as u32;
        // filter flags and limit 2 to just limit 2 and shift into position
        limit |= ((self.limit2_and_flags & 0b00001111) as u32) << 16;
        return limit;
    }
    /// panics if limit is uses the top twelve bits of the u32 (limit is a 20 bytes value)
    pub fn set_limit(&mut self, limit: u32) {
        assert!(limit & 0xFFF00000 == 0);

        // as truncates the top bits of the limit u32
        self.limit = limit as u16;
        // clear the bottom bits of limit2_and_flags (limit 2)
        self.limit2_and_flags &= 0b11110000;

        // pull out bits 16-19 of limit and put them into limit2
        self.limit2_and_flags |= ((limit >> 16) & 0b1111) as u8;
    }

    pub fn get_base(&self) -> u32 {
        let mut base = self.base1 as u32;
        base |= (self.base2 as u32) << 16;
        base |= (self.base3 as u32) << 24;
        return base;
    }

    pub fn set_base(&mut self, base: u32) {
        self.base1 = base as u16;
        self.base2 = (base >> 16) as u8;
        self.base3 = (base >> 24) as u8;
    }

    pub fn get_flags(&self) -> Flags {
        // mask out limit2
        let flags = self.limit2_and_flags & 0b11110000;
        return Flags::from_bits_retain(flags);
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AccessByte: u8 {
        /// Set by the CPU when the segment is accessed. Leave clear
        const accessed = 0b1;
        /// Set to allow read access for code segments.
        ///
        /// Set to allow write access for data segments.
        const readable_writable = 0b10;
        /// Set to make a data segment grow down.
        ///
        /// Set to allow a code segment to be executed at an equal **or lower** privilege level than specified in DPL. Clearing will require an equal DPL.
        const direction_conforming_bit = 0b100;
        /// Set in code segments, clear in data segments.
        const executable = 0b1000;
        /// Clear if the segment is a system segment. Set if the segment is a code or data segment.
        const descriptor_type = 0b10000;
        // The lower bit of the descriptor privilege level (the CPU privilege level of the segment).
        const dpl_low = 0b100000;
        // The higher bit of the descriptor privilege level (the CPU privilege level of the segment).
        const dpl_high = 0b1000000;
        /// Set in any valid segment.
        const present = 0b10000000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Flags: u8{
        // we start at the 5th bit because the flags are in the top half of the byte and one flag is reserved.
        /// Set if the descriptor is a 64 bit code segment. If it is set, `size` should be set as well
        const long_mode_code = 0b100000;
        /// Set in a 32 bit protected mode segment. Clear in a 16 bit protected mode segment or a 64 bit code segment.
        const size = 0b1000000;
        /// Clear for byte granularity (limit is measured in bytes). Set for page granularity (limit is measured in pages).
        const granularity = 0b10000000;
    }
}

#[repr(transparent)]
struct SegmentSelector{
    x: u16,
}

impl SegmentSelector{
    /// Gets the offset of the segment descriptor in the descriptor table in bytes
    pub fn get_offset(&self) -> u16{
        return self.x & 0xFFF8;
    }

    /// Gets the index of the segment descriptor in the descriptor table (measured in segment descriptor entries)
    pub fn get_index(&self) -> usize{
        // divide by 8 and clear the bottom 3 bits
        return self.x >> 3;
    }

    /// Returns true if the segment is selected from the GDT (false if it is from the LDT)
    pub fn uses_gdt(&self) -> bool{
        // The third bit of the segment selector is clear if the GDT is used.
        return &self.x & 0b100 == 0;
    }

    /// Returns the requested privilege level of the segment.
    pub fn privilege_level(&self) -> u8{
        return &self.x & 0b11;
    }
}