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
static mut SUM: i32 = 0;

fn main() {
    unsafe {
        let mut i = 0;
        while i < 3 {
            let x = i * 2;
            if x % 2 == 0 {
                SUM += x;
            } else {
                SUM -= x;
            }
            i += 1;
        }
        let copy = SUM;
        println!("{}", copy);
    }
}

// expected output
// 6
