use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn DataCorruption_Handler() {
    println!("ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler");
    std::io::stdout().flush().unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn SigMismatch_Handler() {
    println!("ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler");
    std::io::stdout().flush().unwrap();
}

// A static initializer can't allocate, unlike C++'s `new int(5)` running as a dynamic
// global constructor, so the allocation itself moves into main(); the pointer variable
// is still what's hardened.
#[unsafe(link_section = "aspis_to_harden")]
static mut P: *mut i32 = std::ptr::null_mut();

fn main() {
    unsafe {
        P = Box::into_raw(Box::new(5));
        *P = 10; // modify the allocated value
        let result = *P;
        drop(Box::from_raw(P));

        println!("Value: {}", result);
    }
}
