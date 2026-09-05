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
pub static mut key: u8 = 0x5A;

extern "C" fn xor_crypt(data: u8, k: u8) -> u8 {
    data ^ k
}

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub extern "C" fn process_buffer(buf: *mut u8, len: usize, k: u8) {
    unsafe {
        for i in 0..len {
            *buf.add(i) = xor_crypt(*buf.add(i), k);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let original: [u8; 11] = *b"HELLOWORLD\0";
    let mut buffer: [u8; 11] = original;

    unsafe {
        process_buffer(buffer.as_mut_ptr(), buffer.len() - 1, key); // Encrypt
        process_buffer(buffer.as_mut_ptr(), buffer.len() - 1, key); // Decrypt
    }

    // `[]` indexing would emit a core::panicking::panic_bounds_check call that
    // only a full rustc-driven link resolves; raw pointer reads don't.
    let mut matches = true;
    let buf_ptr = buffer.as_ptr();
    let orig_ptr = original.as_ptr();
    for i in 0..(original.len() - 1) {
        unsafe {
            if *buf_ptr.add(i) != *orig_ptr.add(i) {
                matches = false;
            }
        }
    }

    unsafe {
        if matches {
            printf(b"SUCCESS\0".as_ptr());
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
// SUCCESS
