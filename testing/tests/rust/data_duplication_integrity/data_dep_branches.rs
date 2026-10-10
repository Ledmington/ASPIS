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
static mut Y: i32 = 0;

fn main() {
    let x = 5;
    unsafe {
        if x > 3 {
            Y = 7;
        } else {
            Y = 9;
        }
        let copy = Y;
        println!("{}", copy);
    }
}

// expected output
// 7
