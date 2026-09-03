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
pub static mut a: i32 = 10;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let b = 20;
    unsafe {
        let c = a + b;
        printf(b"%d\n\0".as_ptr(), c);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
