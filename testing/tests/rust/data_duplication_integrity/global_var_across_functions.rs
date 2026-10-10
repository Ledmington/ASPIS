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
static mut G: i32 = 0;

fn increment() {
    unsafe {
        G += 1;
    }
}

fn print() {
    let copy = unsafe { G };
    println!("{}", copy);
}

fn main() {
    increment();
    increment();
    print();
}

// expected output
// 2
