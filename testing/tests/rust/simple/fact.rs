#![no_std]
#![no_main]

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn DataCorruption_Handler() {
    unsafe {
        printf(b"ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler\n\0".as_ptr());
        fflush(core::ptr::null_mut());
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn SigMismatch_Handler() {
    unsafe {
        printf(b"ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler\n\0".as_ptr());
        fflush(core::ptr::null_mut());
    }
}

// compute n! recursively
#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub extern "C" fn fact(n: u32) -> u64 {
    if n <= 1 { 1 } else { n as u64 * fact(n - 1) }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let result = fact(10);
    unsafe {
        printf(b"%llu\n\0".as_ptr(), result);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
