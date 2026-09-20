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
pub static mut duplicated_global: i32 = 100;

#[unsafe(link_section = "aspis_exclude")]
#[unsafe(no_mangle)]
pub static mut excluded_global: i32 = 200;

// Arrays are not auto-duplicated by DuplicateGlobals unless explicitly marked
// "to_duplicate" (unlike plain scalar globals, which always are).
#[unsafe(link_section = "aspis_to_duplicate")]
#[unsafe(no_mangle)]
pub static mut lookup: [i32; 4] = [1, 2, 3, 4];

extern "C" fn increment(x: i32) -> i32 {
    x + 1
}

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub extern "C" fn multiply_by_two(x: i32) -> i32 {
    x * 2
}

#[unsafe(link_section = "aspis_exclude")]
#[unsafe(no_mangle)]
pub extern "C" fn secret_func(x: i32) -> i32 {
    x - 42
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        let mut val = duplicated_global;
        val = increment(val);
        val = multiply_by_two(val);

        let mut excl = excluded_global;
        excl += 5;

        let secret = secret_func(excl);

        let extra = lookup[0] + lookup[3];

        let result = val + excl + secret + extra;

        if result == 575 {
            printf(b"OK\0".as_ptr());
        } else {
            printf(b"FAIL\0".as_ptr());
        }
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
// OK
