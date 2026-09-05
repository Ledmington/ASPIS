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

// A generic `fn run<F: FnMut()>` would be monomorphized per call site, which clashes
// with `#[no_mangle]` (multiple instantiations can't share one symbol) -- `&mut dyn
// FnMut()` keeps this a single concrete function RustAnnotationBridge can annotate,
// the same role the C++ version's function template played.
#[unsafe(link_section = "aspis_exclude")]
#[unsafe(no_mangle)]
pub extern "C" fn run_no_dup(func: &mut dyn FnMut()) {
    func();
}

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub extern "C" fn run(func: &mut dyn FnMut()) {
    func();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    // Example 1: closure capturing a local variable by reference
    let mut x: i32 = 0;
    let mut incr_x = |val: i32| {
        x += val;
        unsafe {
            printf(b"x incremented by %d\n\0".as_ptr(), val);
        }
    };
    incr_x(5); // can be duplicated safely (x is local to each duplicate)

    unsafe {
        // Example 2: closure capturing a heap-allocated pointer
        let p = malloc(size_of::<i32>()) as *mut i32;
        *p = 10;
        let mut inc_ptr = || {
            *p += 1;
        };
        run_no_dup(&mut inc_ptr); // non-duplicated increment of shared memory
        run(&mut inc_ptr); // duplicated increment of shared memory

        printf(b"Value pointed by p: %d\n\0".as_ptr(), *p);
        free(p as *mut core::ffi::c_void);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
