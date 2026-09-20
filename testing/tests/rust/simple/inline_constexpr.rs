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
pub extern "C" fn print_result(value: i32) {
    unsafe {
        printf(b"%d\n\0".as_ptr(), value);
    }
}

#[inline]
fn square(x: i32) -> i32 {
    x * x
}

const fn get_five() -> i32 {
    5
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let a = 3;
    let b = 4;

    let result1 = square(a); // 9
    let result2 = square(b); // 16
    const C: i32 = get_five(); // 5

    print_result(result1);
    print_result(result2);
    print_result(C);

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
