use core::ptr::{null, null_mut};

use limine::{MemmapEntry, MemoryMapEntryType, NonNullPtr};

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
    fn allocate(&mut self) -> Frame;
    /// Frees the given frame
    fn free(&mut self, frame: Frame);
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

        let mut iter = memory_map
            .iter()
            .filter(|entry| entry.typ == MemoryMapEntryType::Usable)
            .peekable();

        for entry in iter.clone() {
            let mut linked_list_node =
                unsafe { ((entry.base + physical_memory_offset) as *mut LinkedListNode).read() };
            // convert bytes to pages
            linked_list_node.size = entry.len >> 12;
            linked_list_node.next = match iter.peek() {
                Some(next_entry) => {
                    (next_entry.base + physical_memory_offset) as *mut LinkedListNode
                }
                None => null_mut(),
            };
        }

        Self {
            memory_map,
            physical_memory_offset,
            first_node: (physical_start_address + physical_memory_offset) as *mut LinkedListNode,
        }
    }
}

impl FrameAllocator for MemoryMapAllocator<'_> {
    fn allocate(&mut self) -> Frame {
        let mut first_node = if self.first_node != null_mut() {
            unsafe { self.first_node.read() }
        } else {
            panic!("First node is null!");
        };

        // If no pages are available we return a null pointer.
        if first_node.next.is_null() {
            return Frame::from_starting_address(0);
        }
        let first_node_physical_address = (self.first_node as u64) - self.physical_memory_offset;
        if first_node.size == 1 {
            self.first_node = first_node.next; // next = null is checked above
            Frame::from_starting_address(first_node_physical_address)
        } else {
            // If there are multiple frames in this region we take the last one.
            let frame_address = (first_node.size - 1) * 0x1000 + first_node_physical_address;
            first_node.size -= 1;
            Frame::from_starting_address(frame_address)
        }
    }

    fn free(&mut self, frame: Frame) {
        assert_ne!(
            frame.get_starting_address(),
            0,
            "Freed Frame starting address is null!"
        );

        let current_node_ptr = self.first_node;
        let current_node = if current_node_ptr != null_mut() {
            unsafe { current_node_ptr.read() }
        } else {
            panic!("first node is null!");
        };
        loop {
            let current_node_physical_address = (current_node as u64) - self.physical_memory_offset;

            let appends_current = false;
            let prepends_next = false;
            if current_node.size * 0x1000 + current_node_physical_address == frame.starting_address
            {
                // if this region ends directly before this frame, combine it
                appends_current = true;
            }
            if (!current_node.next.is_null())
                && current_node.next - 0x1000 == frame.starting_address
            {
                prepends_next = true;
            }

            if appends_current && prepends_next {
                current_node.size = current_node.size + 1 + current_node.next.size;
                // this is safe because if prepends_next is set, current_node.next cannot be null
                current_node.next = current_node.next.next;
                break;
            } else if appends_current {
                current_node.size += 1;
                break;
            } else if prepends_next {
                // this is safe because if prepends_next is set, current_node.next cannot be null or 0x1000 (because frame 0 cannot be freed)
                let new_next = unsafe { current_node.next.byte_offset(0x1000) };
                unsafe { new_next.write(current_node.next.read()) }
                new_next.size -= 1;
                unsafe {
                    current_node.next.write(LinkedListNode {
                        size: 0,
                        next: null_mut(),
                    })
                };
                current_node.next = new_next;
                break;
            }else{
                // TODO! Make a new linked list node.
            }

            if current_node.next == null_mut() {
                panic!();
            } else {
                current_node = unsafe { current_node.next.read() };
            }
        }
    }
}

#[repr(C)]
struct LinkedListNode {
    /// The size of this region of memory, measured in pages.
    size: u64,
    next: *mut LinkedListNode,
}
