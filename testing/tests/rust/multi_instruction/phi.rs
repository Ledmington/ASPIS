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

fn main() {
    // The process id stands in for a random number: unknown at compile time and non-negative.
    let y = std::process::id() % 2;
    let r = 0;
    let _a = (y != 0) || (r != 0);
    println!("SUCCESS");
}
