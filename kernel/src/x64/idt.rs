use core::{arch::asm, mem::size_of};

use bitflags::bitflags;

use super::gdt::SegmentSelector;

#[derive(Debug)]
#[repr(packed)]
pub struct Idtr {
    pub size: u16,
    pub base: u64,
}

impl Idtr {
    /// loads this IDTR
    /// caller must ensure that self is a valid IDTR
    pub unsafe fn load(&self) {
        asm!("lidt [{idtr}]", idtr = in(reg) self);
    }

    pub fn get() -> Self {
        let x: *mut Idtr;

        unsafe {
            asm!("sidt [{idtr}]", idtr = out(reg) x);
            x.read()
        }
    }

    pub fn from_gate_descriptors(gate_descriptor: &[GateDescriptor]) -> Self {
        Self {
            size: (size_of::<GateDescriptor>() * gate_descriptor.len()) as u16 - 1,
            base: gate_descriptor.as_ptr() as u64,
        }
    }
}

pub enum GateType {
    InterruptGate,
    TrapGate,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct GateDescriptor {
    offset1: u16,
    pub segment_selector: SegmentSelector,
    ist: u8,
    flags: u8,
    offset2: u16,
    offset3: u32,
    reserved: u32,
}

impl GateDescriptor {
    /// Gets the "offset" of this gate descriptor, the entry point of the interrupt service routine.
    pub fn get_offset(&self) -> u64 {
        (self.offset1 as u64) | ((self.offset2 as u64) << 16) | ((self.offset3 as u64) << 32)
    }

    /// Gets the ist this gate descriptor, a 3 bit index into the interrupt stack table stored in the task state segment.
    /// If all bits are zero, the interrupt stack table is not used.
    pub fn get_ist(&self) -> u8 {
        // ist uses the bottom 3 bits of the ist byte
        self.ist & 0b111
    }

    pub fn get_gate_type(&self) -> GateType {
        match self.flags & 0b1111 {
            0xE => GateType::InterruptGate,
            0xF => GateType::TrapGate,
            _ => panic!(),
        }
    }

    /// Gets the lowest privilege level that can access this gate descriptor.
    pub fn get_dpl(&self) -> u8 {
        (self.flags >> 5) & 0b11
    }

    /// Gets the "offset" of this gate descriptor, the entry point of the interrupt service routine.
    pub fn set_offset(&mut self, offset: u64) {
        self.offset1 = offset as u16;
        self.offset2 = (offset >> 16) as u16;
        self.offset3 = (offset >> 32) as u32;
    }

    /// Sets the ist of this gate descriptor, values larger than 7 will panic.
    pub fn set_ist(&mut self, ist: u8) {
        assert_eq!(ist, ist & 0b111);
        self.ist = ist & 0b111;
    }

    /// Sets the type of this gate descriptor.
    pub fn set_gate_type(&mut self, gate_type: GateType) {
        match gate_type {
            GateType::InterruptGate => self.flags |= 0xE,
            GateType::TrapGate => self.flags |= 0xF,
        }
    }

    pub fn set_dpl(&mut self, dpl: u8) {
        assert_eq!(dpl, dpl & 0b11);
        self.flags |= (dpl & 0b11) << 5;
    }

    pub fn set_present(&mut self, present: bool) {
        if present {
            self.flags |= 0b10000000;
        } else {
            self.flags &= 0b01111111;
        }
    }

    /// Creates a null gate descriptor (this is an invalid descriptor).
    pub fn create_null_descriptor() -> Self {
        Self {
            offset1: 0,
            segment_selector: SegmentSelector { x: 0 },
            ist: 0,
            flags: 0,
            offset2: 0,
            offset3: 0,
            reserved: 0,
        }
    }

    pub fn create_exception_handler(handler_function: extern "x86-interrupt" fn(), cs: SegmentSelector) -> Self {
        let mut exception_handler = Self::create_null_descriptor();
        exception_handler.set_offset(handler_function as *const () as u64);
        exception_handler.segment_selector = cs;
        // trap gates are used for exceptions
        exception_handler.set_gate_type(GateType::TrapGate);
        // we don't need to set ist or dpl
        exception_handler.set_present(true);

        exception_handler
    }
}
