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

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub extern "C" fn foo() -> i32 {
    let c = 12;
    let d = 13;
    c + d
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let a = 10;
    let b = 20;
    let c = foo();
    unsafe {
        printf(b"foo() %d\n\0".as_ptr(), c);
    }
    if a > b { 1 } else { 0 }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
