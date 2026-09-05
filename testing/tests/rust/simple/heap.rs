#![no_std]
#![no_main]

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn free(ptr: *mut core::ffi::c_void);
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

// A global initializer can't call malloc (not const-evaluable), unlike C++'s `new int(5)`
// running as a dynamic global constructor, so the allocation itself moves into main();
// the pointer variable is still what's hardened.
#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub static mut p: *mut i32 = core::ptr::null_mut();

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        p = malloc(size_of::<i32>()) as *mut i32;
        *p = 5;
        *p = 10; // modify the allocated value
        let result = *p;
        free(p as *mut core::ffi::c_void);

        printf(b"Value: %d\n\0".as_ptr(), result);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
