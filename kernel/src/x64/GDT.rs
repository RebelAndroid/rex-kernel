use core::arch::asm;

#[derive(Debug)]
#[repr(packed)]
pub struct GDTR {
    limit: u16,
    base: u64,
}

impl GDTR {
    /// loads this GDTR
    /// caller must ensure that self is a valid GDTR
    pub unsafe fn load(&self) {
        asm!("lgdt [{gdtr}]", gdtr = in(reg) &self);
    }

    pub fn get() -> Self {
        let x: *mut GDTR;

        unsafe {
            asm!("sgdt [{gdtr}]", gdtr = out(reg) x);
            x.read()
        }
    }
}
