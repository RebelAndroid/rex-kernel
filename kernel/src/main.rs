#![no_std]
#![no_main]

use core::arch::asm;

use core::fmt::Write;

use uart_16550::SerialPort;

static FRAMEBUFFER_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static MEMORY_MAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static HHDM_REQUEST: limine::HhdmRequest = limine::HhdmRequest::new(0);

mod x64;

use x64::gdt::Gdtr;

use x64::registers::{get_cs, get_ds, get_es, get_ss};

use crate::x64::registers::{get_fs, get_gs};

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();

    // Ensure we got a framebuffer.
    let framebuffer = if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response().get() {
        if framebuffer_response.framebuffer_count < 1 {
            writeln!(serial_port, "No framebuffers found!");
            halt_loop();
        }

        let _ = writeln!(serial_port, "framebuffer found");

        // Get the first framebuffer's information.
        &framebuffer_response.framebuffers()[0]
    } else {
        let _ = writeln!(serial_port, "Framebuffer response not received!");
        halt_loop();
    };

    if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response().get() {
        //let _ = writeln!(serial_port, "memory map: {:x?}", memory_map_response.memmap());
    }

    for i in 0..100_usize {
        // Calculate the pixel offset using the framebuffer information we obtained above.
        // We skip `i` scanlines (pitch is provided in bytes) and add `i * 4` to skip `i` pixels forward.
        let pixel_offset = i * framebuffer.pitch as usize + i * 4;

        // Write 0xFFFFFFFF to the provided pixel offset to fill it white.
        // We can safely unwrap the result of `as_ptr()` because the framebuffer address is
        // guaranteed to be provided by the bootloader.
        unsafe {
            *(framebuffer
                .address
                .as_ptr()
                .unwrap()
                .offset(pixel_offset as isize) as *mut u32) = 0xFFFFFFFF;
        }
    }

    let current_gdtr: Gdtr = Gdtr::get();
    let _ = writeln!(serial_port, "current gdtr: {:x?}", current_gdtr);

    let physical_memory_offset = if let Some(hhdm_response) = HHDM_REQUEST.get_response().get() {
        let _ = writeln!(serial_port, "HHDM response: {:x}", hhdm_response.offset);
        hhdm_response.offset
    } else {
        panic!("HHDM response not received!");
    };

    let _ = writeln!(
        serial_port,
        "GDT physical address: {:x}",
        current_gdtr.base - physical_memory_offset
    );

    let mut index = 0;

    while let Some(segment_descriptor) = current_gdtr.get_segment_descriptor(index) {
        writeln!(serial_port, "{:x?}", segment_descriptor).unwrap();
        index += 1;
    }

    let cs = get_cs();
    writeln!(serial_port, "cs: {:?}", cs);
    let ds = get_ds();
    writeln!(serial_port, "ds: {:?}", ds);
    let es = get_es();
    writeln!(serial_port, "es: {:?}", es);
    let ss = get_ss();
    writeln!(serial_port, "ss: {:?}", ss);
    let fs = get_fs();
    writeln!(serial_port, "fs: {:?}", fs);
    let gs = get_gs();
    writeln!(serial_port, "gs: {:?}", gs);




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
