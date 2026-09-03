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

// Simulates a resource (like a file) using RAII: `Drop` stands in for the C++
// destructor. No annotation here (matches the original), since this test is about
// ASPIS not breaking scope-based cleanup, not about hardening a particular value.
struct FakeFileHandler;

impl FakeFileHandler {
    fn new() -> Self {
        unsafe {
            printf(b"Handler created\n\0".as_ptr());
        }
        FakeFileHandler
    }
}

impl Drop for FakeFileHandler {
    fn drop(&mut self) {
        unsafe {
            printf(b"File closed\n\0".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let _handler = FakeFileHandler::new();
    // _handler dropped automatically at end of scope
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
