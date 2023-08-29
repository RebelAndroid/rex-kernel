#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::arch::asm;

use core::fmt::Write;

use spin::Mutex;
use uart_16550::SerialPort;

static FRAMEBUFFER_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static MEMORY_MAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static HHDM_REQUEST: limine::HhdmRequest = limine::HhdmRequest::new(0);

mod x64;
use crate::pmm::{MemoryMapAllocator, FrameAllocator};
use crate::x64::idt::{Idt};
use crate::x64::registers::get_cs;

mod pmm;

static debug_serial_port: Mutex<SerialPort> = Mutex::new(unsafe{
    SerialPort::new(0x3F8)
});

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    debug_serial_port.lock().init();

    // Ensure we got a framebuffer.
    let framebuffer = if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if framebuffer_response.framebuffer_count < 1 {
            writeln!(debug_serial_port.lock(), "No framebuffers found!");
            halt_loop();
        }

        let _ = writeln!(debug_serial_port.lock(), "framebuffer found");

        // Get the first framebuffer's information.
        &framebuffer_response.framebuffers()[0]
    } else {
        let _ = writeln!(debug_serial_port.lock(), "Framebuffer response not received!");
        halt_loop();
    };

    let memory_map = if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response().get() {
        memory_map_response
    }else{
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
        //let _ = writeln!(debug_serial_port.lock(), "HHDM response: {:x}", hhdm_response.offset);
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

    for i in memory_map.memmap(){
        writeln!(debug_serial_port.lock(), "memory map entry: {:x?}", i);
    }

    let mut frame_allocator = MemoryMapAllocator::new(memory_map.memmap(), physical_memory_offset);
    let frame = frame_allocator.allocate();
    writeln!(debug_serial_port.lock(), "allocated frame: {:x?}", frame);

    writeln!(debug_serial_port.lock(), "finished, halting");    
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
        "Page fault! Error code: {}, Address: {}",
        error_code, address
    );
}

extern "x86-interrupt" fn general_protection_fault(_: u64, error_code: u64) {
    panic!("Page fault! Error code: {},", error_code);
}

extern "x86-interrupt" fn double_fault(_: u64, error_code: u64) -> ! {
    panic!("Double fault! Error code: {}", error_code);
}
