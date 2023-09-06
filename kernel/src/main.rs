#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(dead_code)]
use core::arch::asm;

use core::fmt::Write;

use generic_once_cell::OnceCell;
use spin::Mutex;
use uart_16550::SerialPort;

static FRAMEBUFFER_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static MEMORY_MAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static HHDM_REQUEST: limine::HhdmRequest = limine::HhdmRequest::new(0);

static DIRECT_MAP_START: OnceCell<Mutex<()>, u64> = OnceCell::new();
static PHYSICAL_MEMORY_SIZE: OnceCell<Mutex<()>, u64> = OnceCell::new();

mod x64;
use crate::pmm::MemoryMapAllocator;
use crate::x64::idt::Idt;
use crate::x64::registers::{get_cr3, get_cs};

mod pmm;

mod memory;

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

    for i in 0..100_usize {
        let pixel_offset = i * framebuffer.pitch as usize + i * 4;
        unsafe {
            *(framebuffer
                .address
                .as_ptr()
                .unwrap()
                .offset(pixel_offset as isize) as *mut u32) = 0xFFFFFFFF;
        }
    }

    let physical_memory_offset = if let Some(hhdm_response) = HHDM_REQUEST.get_response().get() {
        unsafe { DIRECT_MAP_START.set(hhdm_response.offset).unwrap() }
        hhdm_response.offset
    } else {
        panic!("HHDM response not received!");
    };

    let cs = get_cs();

    // TODO: make idt a mut static
    let mut idt = Idt::new();
    idt.set_page_fault_handler(page_fault, cs);
    idt.set_general_protection_fault_handler(general_protection_fault, cs);
    idt.set_double_fault_handler(double_fault, cs);

    let idtr = idt.get_idtr();
    idtr.load();

    let mut frame_allocator = MemoryMapAllocator::new(memory_map.memmap(), physical_memory_offset);

    let cr3 = get_cr3();
    writeln!(DEBUG_SERIAL_PORT.lock(), "cr3: {:x}", cr3.address()).unwrap();
    writeln!(
        DEBUG_SERIAL_PORT.lock(),
        "PML4: {:x?}",
        cr3.pml4(physical_memory_offset)
    )
    .unwrap();
    writeln!(
        DEBUG_SERIAL_PORT.lock(),
        "physical memory offset: {:x}",
        physical_memory_offset
    )
    .unwrap();

    let current_pml4 = cr3.pml4(physical_memory_offset);
    let (new_pml4, new_pml4_physical_address) =
        current_pml4.deep_copy(&mut frame_allocator, physical_memory_offset);

    writeln!(DEBUG_SERIAL_PORT.lock(), "finished, halting").unwrap();
    halt_loop();
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

extern "x86-interrupt" fn page_fault(_: u64, error_code: u64) {
    let address: u64;
    // The x86-interrupt calling convention helpfully pops the error code for us, but we still need to read cr2 to find the virtual address of the page fault
    unsafe {
        asm!(
        "mov {addr}, cr2",
        addr = out(reg) address,
        )
    };
    panic!(
        "Page fault! Error code: {}, Address: {:x}",
        error_code, address
    );
}

extern "x86-interrupt" fn general_protection_fault(_: u64, error_code: u64) {
    panic!("Page fault! Error code: {},", error_code);
}

extern "x86-interrupt" fn double_fault(_: u64, error_code: u64) -> ! {
    panic!("Double fault! Error code: {}", error_code);
}
