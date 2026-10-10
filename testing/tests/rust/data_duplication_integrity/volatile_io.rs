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

// Simulate memory-mapped I/O: not annotated, read through a volatile load so it
// isn't folded to a constant before ASPIS ever sees it.
static mut IO_PORT: i32 = 42;

fn main() {
    let x = unsafe { std::ptr::read_volatile(&raw const IO_PORT) };
    println!("{}", x);
}

// expected output
// 42
