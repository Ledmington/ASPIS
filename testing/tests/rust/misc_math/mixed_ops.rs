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
pub static mut res: f32 = 0.0;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let i: i32 = 5;
    let f: f32 = 2.5;
    let c: i8 = 3;
    let l: i64 = 4;

    unsafe {
        // 5 + 2.5 + 3 + 4 = 14.5
        res = i as f32 + f + c as f32 + l as f32;
        // C variadic calls always promote float args to double.
        printf(b"%.1f\0".as_ptr(), res as f64);
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
// 14.5
