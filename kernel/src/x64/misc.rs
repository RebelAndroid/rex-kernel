use core::arch::asm;

/// Stops interrupts then issues a halt instruction repeatedly
pub fn halt_loop() -> !{
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}