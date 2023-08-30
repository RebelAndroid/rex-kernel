
// all uses of cpuid in this module will cause a invalid opcode exception if cpuid is not supported

use core::arch::{asm, x86_64::__cpuid};

/// Gets the vendor string of the processor
pub fn get_vendor_string() -> [u8; 12]{
    let cpuid_result =  unsafe{ __cpuid(0) };
    let ebx = cpuid_result.ebx;
    let edx = cpuid_result.edx;
    let ecx = cpuid_result.ecx;

    let mut output : [u8; 12] = [0; 12];

    output[0] = ebx.to_le_bytes()[0];
    output[1] = ebx.to_le_bytes()[1];
    output[2] = ebx.to_le_bytes()[2];
    output[3] = ebx.to_le_bytes()[3];

    output[4] = edx.to_le_bytes()[0];
    output[5] = edx.to_le_bytes()[1];
    output[6] = edx.to_le_bytes()[2];
    output[7] = edx.to_le_bytes()[3];

    output[8]  = ecx.to_le_bytes()[0];
    output[9]  = ecx.to_le_bytes()[1];
    output[10] = ecx.to_le_bytes()[2];
    output[11] = ecx.to_le_bytes()[3];

    output
}