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

// Print function (non-duplicated)
#[unsafe(link_section = "aspis_exclude")]
#[unsafe(no_mangle)]
pub extern "C" fn print_result(value: i32) {
    unsafe {
        printf(b"Result: %d\n\0".as_ptr(), value);
    }
}

// Example of a function template
fn my_max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// Example of a simple accumulator struct template
struct Accumulator<T> {
    sum: T,
}

impl<T: core::ops::AddAssign> Accumulator<T> {
    fn add(&mut self, value: T) {
        self.sum += value; // duplicated safely
    }
}

impl<T: Copy> Accumulator<T> {
    fn total(&self) -> T {
        self.sum
    }
}

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub static mut acc: Accumulator<i32> = Accumulator { sum: 0 };

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let x = 42;
    let y = 17;
    let max_val = my_max(x, y); // 42

    unsafe {
        acc.add(5);
        acc.add(10);
    }
    let sum_val = unsafe { acc.total() }; // 15

    print_result(max_val);
    print_result(sum_val);
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
