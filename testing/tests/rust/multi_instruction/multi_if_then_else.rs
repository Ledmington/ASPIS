#![no_std]
#![no_main]

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
    fn rand() -> i32;
    fn srand(seed: u32);
    fn time(t: *mut i64) -> i64;
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

const MAX: i32 = 1024;

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub static mut r: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        srand(time(core::ptr::null_mut()) as u32);
        // `%`/`wrapping_rem` both need core's checked-remainder panic glue (for the
        // divide-by-zero case) even for a compile-time-nonzero divisor, and that glue
        // isn't linked into this freestanding binary. MAX is a power of two and rand()
        // is always non-negative (POSIX), so a bitmask is an exact, panic-free substitute.
        r = (rand() & (MAX - 1)) + 200;
        if r > 200 {
            printf(b"r > 200\n\0".as_ptr());
        } else if r > 100 {
            printf(b"100 < r < 200\n\0".as_ptr());
        } else if r > 50 {
            printf(b"50 < r < 100\n\0".as_ptr());
        } else {
            printf(b"0 < r < 50  \n\0".as_ptr());
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
