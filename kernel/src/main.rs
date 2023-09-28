#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(pointer_byte_offsets)]
#![feature(offset_of)]
#![allow(dead_code)]
#![allow(unused_imports)]

use core::arch::asm;

use core::fmt::Write;
use core::mem::{size_of, offset_of};

use acpi::root::RSDP32Bit;
use generic_once_cell::OnceCell;
use memory::{DirectMappedAddress};
use spin::Mutex;
use uart_16550::SerialPort;
use x64::idt::PageFaultErrorCode;

static FRAMEBUFFER_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static MEMORY_MAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static HHDM_REQUEST: limine::HhdmRequest = limine::HhdmRequest::new(0);
static RSDP_REQUEST: limine::RsdpRequest = limine::RsdpRequest::new(0);

static DIRECT_MAP_START: OnceCell<Mutex<()>, u64> = OnceCell::new();
static PHYSICAL_MEMORY_SIZE: OnceCell<Mutex<()>, u64> = OnceCell::new();

static FRAME_ALLOCATOR: OnceCell<Mutex<()>, Mutex<MemoryMapAllocator>> = OnceCell::new();

mod x64;
use crate::acpi::fadt::{FADT, GenericAddressStructure};
use crate::acpi::root::{RSDP64Bit};
use crate::memory::VirtualAddress;
use crate::pmm::MemoryMapAllocator;
use crate::x64::idt::Idt;
use crate::x64::registers::{get_cr3, get_cs};

mod pmm;

mod memory;

mod acpi;

static DEBUG_SERIAL_PORT: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    DEBUG_SERIAL_PORT.lock().init();

    // Ensure we got a framebuffer.
    let framebuffer = if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if framebuffer_response.framebuffer_count < 1 {
            panic!("No framebuffers found!");
        }
        // Get the first framebuffer's information.
        &framebuffer_response.framebuffers()[0]
    } else {
        panic!("Framebuffer response not received!");
    };

    let memory_map = if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response().get() {
        let mut highest_address: u64 = 0;
        for entry in memory_map_response.memmap() {
            highest_address = u64::max(highest_address, entry.base + entry.len);
        }
        if highest_address == 0 {
            panic!("Error in memory map!");
        } else {
            unsafe { PHYSICAL_MEMORY_SIZE.set(highest_address).unwrap() }
        }
        memory_map_response
    } else {
        panic!("Memory map not received!");
    };

    for entry in memory_map.memmap() {
        writeln!(
            DEBUG_SERIAL_PORT.lock(),
            "memory map entry: {:x?}, last_frame: {:x}",
            entry,
            entry.base + entry.len - 0x1000
        );
    }

    let physical_memory_offset = if let Some(hhdm_response) = HHDM_REQUEST.get_response().get() {
        unsafe { DIRECT_MAP_START.set(hhdm_response.offset).unwrap() }
        hhdm_response.offset
    } else {
        panic!("HHDM response not received!");
    };

    let rsdp_ptr = if let Some(rsdp_response) = RSDP_REQUEST.get_response().get() {
        rsdp_response.address.as_ptr().unwrap() as *mut RSDP32Bit
    } else {
        panic!("RSDP response not received or invalid!");
    };

    let cs = get_cs();

    // TODO: make idt a mut static
    let mut idt = Idt::new();
    idt.set_page_fault_handler(page_fault, cs);
    idt.set_general_protection_fault_handler(general_protection_fault, cs);
    idt.set_double_fault_handler(double_fault, cs);

    let idtr = idt.get_idtr();
    idtr.load();

    FRAME_ALLOCATOR
        .set(Mutex::new(MemoryMapAllocator::new(
            memory_map.memmap(),
            physical_memory_offset,
        )))
        .unwrap();

    let cr3 = get_cr3();
    writeln!(DEBUG_SERIAL_PORT.lock(), "cr3: {:x}", cr3.address()).unwrap();

    writeln!(
        DEBUG_SERIAL_PORT.lock(),
        "physical memory offset: {:x}",
        physical_memory_offset
    )
    .unwrap();

    let current_pml4 = cr3.pml4();

    let rsdp = unsafe { &mut *rsdp_ptr };
    assert!(rsdp.checksum());
    let rsdp = if rsdp.revision() == 2 {
        unsafe { &mut *(rsdp_ptr as *mut RSDP64Bit) }
    } else {
        panic!("expected ACPI revision 2");
    };
    assert!(rsdp.checksum());
    let xsdt = rsdp.get_xsdt();
    let xsdt = unsafe { &mut *xsdt };
    assert!(xsdt.checksum());

    let madt = xsdt.get_madt().unwrap();

    GenericAddressStructure::check_offsets();
    FADT::check_offsets();
    let fadt = xsdt.get_fadt().unwrap();
    writeln!(DEBUG_SERIAL_PORT.lock(), "fadt: {:?}", fadt);

    writeln!(DEBUG_SERIAL_PORT.lock(), "finished, halting").unwrap();
    halt_loop();
}

/// Pauses execution (counts really high)
fn pause() {
    writeln!(DEBUG_SERIAL_PORT.lock(), "pausing").unwrap();
    let mut x: i32 = 0;
    while x < 500000000 {
        x += 1;
    }
    writeln!(DEBUG_SERIAL_PORT.lock(), "unpausing").unwrap();
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    // the serial port is used elsewhere, but that doesn't matter for a panic handler
    // (there is no issue interrupting it because we aren't coming back to it)
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();

    let _ = writeln!(serial_port, "\nPANIC!: {}", info);

    halt_loop();
}

fn halt_loop() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn page_fault(_: u64, error_code: PageFaultErrorCode) {
    let address: u64;
    // The x86-interrupt calling convention helpfully pops the error code for us, but we still need to read cr2 to find the virtual address of the page fault
    unsafe {
        asm!(
        "mov {addr}, cr2",
        addr = out(reg) address,
        )
    };
    let direct_address = DirectMappedAddress::try_from_virtual(VirtualAddress::create(address));
    let physical_address = match direct_address{
        Some(direct_mapped_address) => direct_mapped_address.get_physical_address().get_address(),
        None => 1,
    };
    panic!(
        "Page fault! Error code: {:?}, Address: {:x}, Phyiscal Address: {:x}",
        error_code, address, physical_address
    );
}

extern "x86-interrupt" fn general_protection_fault(_: u64, error_code: u64) {
    panic!("Page fault! Error code: {},", error_code);
}

extern "x86-interrupt" fn double_fault(_: u64, error_code: u64) -> ! {
    panic!("Double fault! Error code: {}", error_code);
}

/// Generates a breakpoint interrupt
pub fn breakpoint() {
    unsafe {
        asm!("int3");
    }
}

