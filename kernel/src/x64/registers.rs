use core::arch::asm;

use super::gdt::SegmentSelector;

/// Reads the value of the cs register
pub fn get_cs() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, cs", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the ds register
pub fn get_ds() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, ds", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the ss register
pub fn get_ss() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, ss", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the es register
pub fn get_es() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, es", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the fs register
pub fn get_fs() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, fs", output = out(reg) x) }
    SegmentSelector { x }
}

/// Reads the value of the gs register
pub fn get_gs() -> SegmentSelector {
    let x: u16;
    unsafe { asm!("mov {output:x}, gs", output = out(reg) x) }
    SegmentSelector { x }
}