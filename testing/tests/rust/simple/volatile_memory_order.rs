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

#[unsafe(link_section = "aspis_to_harden")]
static mut FLAG: i32 = 0;

fn writer() {
    unsafe {
        std::ptr::write_volatile(&raw mut FLAG, 42);
    }
}

fn reader() {
    let local = unsafe { std::ptr::read_volatile(&raw const FLAG) };
    println!("{}", local);
}

fn main() {
    writer(); // Writes 42 to the volatile variable
    reader(); // Reads it back and prints it
}
