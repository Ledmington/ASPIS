#![no_main]

// Needed to compile a source file without a Cargo.toml
extern crate aspis_rust_annotations;

use aspis_rust_annotations::aspis;

use std::os::raw::{c_char, c_int, c_void};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

#[aspis(to_harden)]
pub static mut SUM: i32 = 0;

#[no_mangle]
extern "C" fn DataCorruption_Handler() {
    unsafe {
        printf(
            b"ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler\n\0".as_ptr() as *const c_char,
        );
        fflush(std::ptr::null_mut());
    }
}

#[no_mangle]
extern "C" fn SigMismatch_Handler() {
    unsafe {
        printf(b"ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler\n\0".as_ptr() as *const c_char);
        fflush(std::ptr::null_mut());
    }
}

#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut i = 0;

    while i < 5 {
        if i == 1 {
            i += 1;
            continue;
        }
        if i == 3 {
            break;
        }
        unsafe {
            SUM += i;
        }
        i += 1;
    }

    unsafe {
        let copy = SUM;
        printf(b"%d\n\0".as_ptr() as *const c_char, copy);
    }
    0
}

// expected output
// 2
