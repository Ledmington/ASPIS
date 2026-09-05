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
pub static mut sum: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        // A manual `while` loop, not `for i in 0..3`: the Range iterator pulls in
        // core::iter safety-check internals that only a full rustc-driven link
        // resolves, and that --inter-rasm's block splitting can expose even here.
        let mut i = 0;
        while i < 3 {
            let x = i * 2;
            // `x % 2` on a non-constant divisor would emit a
            // core::panicking::panic_const_rem_overflow call that only a full
            // rustc-driven link resolves; `x & 1` is bitwise, so it doesn't.
            if x & 1 == 0 {
                sum += x;
            } else {
                sum -= x;
            }
            i += 1;
        }
        printf(b"%d\0".as_ptr(), sum);
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
// 6
