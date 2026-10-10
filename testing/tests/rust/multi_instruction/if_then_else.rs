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
static mut X: i32 = 1000;

fn main() {
    unsafe {
        X = X + 1;
        if X < 10 {
            X = X * 10;
            let copy = X;
            println!("{}", copy);
        } else {
            let copy = X;
            println!("{}", copy);
        }
    }
}
