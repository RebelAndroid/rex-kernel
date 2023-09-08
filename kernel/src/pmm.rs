use core::ptr::null_mut;

use limine::{MemmapEntry, MemoryMapEntryType, NonNullPtr};

use crate::{memory::PhysicalAddress, DEBUG_SERIAL_PORT};

use core::fmt::Write;

#[derive(Debug)]
pub struct Frame {
    starting_address: u64,
}
impl Frame {
    pub fn from_starting_address(physical_address: PhysicalAddress) -> Self {
        assert!(
            physical_address.is_frame_aligned(),
            "Attempted to create Frame with unaligned starting address."
        );
        assert_ne!(
            physical_address.get_address(),
            0,
            "Attempted to create null frame!"
        );
        Self {
            starting_address: physical_address.get_address(),
        }
    }

    pub fn get_starting_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.starting_address)
    }
}

pub trait FrameAllocator {
    /// Allocates a new frame
    fn allocate(&mut self) -> Option<Frame>;
    /// Frees the given frame
    fn free(&mut self, frame: Frame);
}

#[derive(Debug)]
pub struct MemoryMapAllocator {
    /// The memory map provided by the bootloader
    /// The address at which physical memory is mapped
    physical_memory_offset: u64,
    /// The physical address of the first node in the linked list.
    first_node: *mut LinkedListNode,
}

// This is probably fine because first_node shouldn't be aliased
unsafe impl Send for MemoryMapAllocator{}

impl MemoryMapAllocator {
    pub fn new(memory_map: &[NonNullPtr<MemmapEntry>], physical_memory_offset: u64) -> Self {
        let mut physical_start_address = 0;
        for memory_map_entry in memory_map {
            if memory_map_entry.typ == MemoryMapEntryType::Usable {
                physical_start_address = memory_map_entry.base;
                break;
            }
        }

        let mut first_node: *mut LinkedListNode = null_mut();

        let mut iter = memory_map
            .iter()
            .filter(|entry| entry.typ == MemoryMapEntryType::Usable);

        for entry in iter {
            let physical_address = entry.base;
            let size = entry.len >> 12; // convert bytes to pages
            let virtual_address = physical_address + physical_memory_offset;
            let new_node = unsafe {
                assert_ne!(entry.base, 0);
                (virtual_address as *mut LinkedListNode).write(LinkedListNode {
                    size,
                    next: null_mut(),
                });
                virtual_address as *mut LinkedListNode
            };

            if first_node.is_null() {
                first_node = new_node;
            } else {
                unsafe { *new_node }.next = first_node;
                first_node = new_node;
            }
        }

        Self {
            physical_memory_offset,
            first_node,
        }
    }
}

impl FrameAllocator for MemoryMapAllocator {
    fn allocate(&mut self) -> Option<Frame> {
        let output = if self.first_node.is_null() {
            None
        } else {
            // This is safe because no other references to first_node can exist
            let first_node = unsafe { &mut *self.first_node };
            if first_node.size == 1 {
                let frame = Frame::from_starting_address(PhysicalAddress::new(
                    self.first_node as u64 - self.physical_memory_offset,
                ));
                // remove self.first_node and make the next node the new first node
                self.first_node = first_node.next;
                // clear the node in the returned page
                first_node.size = 0;
                first_node.next = null_mut();
                Some(frame)
            } else {
                first_node.size -= 1;
                Some(Frame::from_starting_address(PhysicalAddress::new(
                    self.first_node as u64 - self.physical_memory_offset + 0x1000 * first_node.size,
                )))
            }
        };
        writeln!(DEBUG_SERIAL_PORT.lock(), "allocated physical frame: {:x?}", output);
        output
    }

    fn free(&mut self, frame: Frame) {
        todo!()
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct LinkedListNode {
    /// The size of this region of memory, measured in pages.
    size: u64,
    next: *mut LinkedListNode,
}
