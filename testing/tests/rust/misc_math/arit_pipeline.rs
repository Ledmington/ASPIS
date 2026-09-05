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
pub static mut modulo: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let a = 5;
    let b = 3;
    let sum = a + b;
    let diff = sum - 2;
    let prod = diff * 4;
    let quot = prod / 3;
    unsafe {
        modulo = quot % 5;
        // expected result: ((((5+3)-2)*4)/3)%5 = (6*4)/3 = 24/3 = 8 % 5 = 3
        printf(b"%d\0".as_ptr(), modulo);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

// expected output
// 3
