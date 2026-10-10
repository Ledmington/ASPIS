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

const MAX: i32 = 1024;

#[unsafe(link_section = "aspis_to_harden")]
static mut R: i32 = 0;

fn main() {
    // The process id stands in for a random number: unknown at compile time and non-negative.
    let random = std::process::id() as i32;
    unsafe {
        R = random % MAX + 200;
        if R > 200 {
            println!("r > 200");
        } else if R > 100 {
            println!("100 < r < 200");
        } else if R > 50 {
            println!("50 < r < 100");
        } else {
            println!("0 < r < 50  ");
        }
    }
}
