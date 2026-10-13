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

// compute n! recursively
#[unsafe(link_section = "aspis_to_harden")]
fn fact(n: u32) -> u64 {
    if n <= 1 { 1 } else { n as u64 * fact(n - 1) }
}

fn main() {
    let result = fact(10);
    println!("{}", result);
}
