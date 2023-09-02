use core::ptr::null_mut;

use limine::{MemmapEntry, MemoryMapEntryType, NonNullPtr};

#[derive(Debug)]
pub struct Frame {
    starting_address: u64,
}
impl Frame {
    pub fn from_starting_address(starting_address: u64) -> Self {
        assert!(starting_address & 0xFFF == 0);
        Self {
            starting_address: starting_address,
        }
    }

    pub fn get_starting_address(&self) -> u64 {
        self.starting_address
    }
}

pub trait FrameAllocator {
    /// Allocates a new frame
    fn allocate(&mut self) -> Option<Frame>;
    /// Frees the given frame
    fn free(&mut self, frame: Frame);
}

// implements FrameAllocator for an optional frame allocator
// Some uses the inner FrameAllocator, None returns null
impl<T> FrameAllocator for Option<T>
where
    T: FrameAllocator,
{
    fn allocate(&mut self) -> Option<Frame> {
        match self{
            Some(inner) => inner.allocate(),
            None => None,
        }
    }

    fn free(&mut self, frame: Frame) {
        match  self {
            Some(inner) => inner.free(frame),
            None => {},
        }
    }
}

pub struct MemoryMapAllocator<'a> {
    /// The memory map provided by the bootloader
    memory_map: &'a [NonNullPtr<MemmapEntry>],
    /// The address at which physical memory is mapped
    physical_memory_offset: u64,
    /// The physical address of the first node in the linked list.
    first_node: *mut LinkedListNode,
}

impl<'a> MemoryMapAllocator<'a> {
    pub fn new(memory_map: &'a [NonNullPtr<MemmapEntry>], physical_memory_offset: u64) -> Self {
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
            memory_map,
            physical_memory_offset,
            first_node,
        }
    }
}

impl FrameAllocator for MemoryMapAllocator<'_> {
    fn allocate(&mut self) -> Option<Frame> {
        if self.first_node.is_null() {
            None
        } else {
            // safe because of null check
            let mut first_node = unsafe { *self.first_node };
            if first_node.size == 1 {
                let frame = Frame::from_starting_address(
                    self.first_node as u64 - self.physical_memory_offset,
                );
                // remove self.first_node and make the next node the new first node
                self.first_node = first_node.next;
                // clear the node in the returned page
                first_node.size = 0;
                first_node.next = null_mut();
                Some(frame)
            } else {
                first_node.size -= 1;
                Some(Frame::from_starting_address(
                    self.first_node as u64 - self.physical_memory_offset + 0x1000 * first_node.size,
                ))
            }
        }
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
