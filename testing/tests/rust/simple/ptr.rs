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

#[repr(C)]
struct Pair {
    a: i32,
    b: i32,
}

// Helper to print two pointer values (non-duplicated)
#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub extern "C" fn print_pointers(p1: *const i32, p2: *const i32) {
    unsafe {
        printf(b"Value pointed by p1: %d\0".as_ptr(), *p1);
        if !p2.is_null() {
            printf(b", Value pointed by p2: %d\0".as_ptr(), *p2);
        }
        printf(b"\n\0".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    unsafe {
        // Allocate a small array on the heap
        let buffer = malloc(2 * size_of::<i32>()) as *mut i32;
        *buffer.add(0) = 100;
        *buffer.add(1) = 200;
        // Use a single pointer + offset instead of two aliases
        let base = buffer;
        let _second_value = *base.add(1);

        // Example struct with two distinct heap allocations
        let obj = malloc(size_of::<Pair>()) as *mut Pair;
        (*obj).a = 1;
        (*obj).b = 2;

        // Example of copying data instead of aliasing
        let p1 = malloc(size_of::<i32>()) as *mut i32;
        *p1 = 42;
        let p2 = malloc(size_of::<i32>()) as *mut i32; // copy the value
        *p2 = *p1;

        // Print the results (non-duplicated)
        print_pointers(p1, p2);

        free(buffer as *mut core::ffi::c_void);
        free(obj as *mut core::ffi::c_void);
        free(p1 as *mut core::ffi::c_void);
        free(p2 as *mut core::ffi::c_void);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
