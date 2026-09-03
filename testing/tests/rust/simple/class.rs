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

struct MyClass {
    a: i32,
    b: i32,
}

impl MyClass {
    fn sum(&self) -> i32 {
        self.a + self.b
    }

    fn print(&self) {
        unsafe {
            printf(b"%d, %d\n\0".as_ptr(), self.a, self.b);
        }
    }
}

// Derived "class": composition stands in for C++ inheritance, and an inherent method
// overriding the base's `print` stands in for the virtual override (main() below calls
// each concretely-typed object's own `print`, never through a base-class pointer, so no
// dynamic dispatch is actually exercised -- a `dyn Trait` vtable would add complexity
// without changing what's tested).
struct DerivedClass {
    base: MyClass,
    c: i32,
}

impl DerivedClass {
    fn print(&self) {
        unsafe {
            printf(b"%d, %d, %d\n\0".as_ptr(), self.base.a, self.base.b, self.c);
        }
    }
}

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub static mut derived_obj: DerivedClass = DerivedClass {
    base: MyClass { a: 3, b: 6 },
    c: 9,
};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    // Test class and member function
    let my_obj = MyClass { a: 5, b: 7 };
    unsafe {
        printf(b"%d\n\0".as_ptr(), my_obj.sum());
    }
    my_obj.print();

    // Test derived class with overridden "virtual" function
    unsafe {
        derived_obj.print();
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
