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
static mut MODULO: i32 = 0;

fn main() {
    let a = 5;
    let b = 3;
    let sum = a + b;
    let diff = sum - 2;
    let prod = diff * 4;
    let quot = prod / 3;
    unsafe {
        MODULO = quot % 5;
        // expected result: ((((5+3)-2)*4)/3)%5 = (6*4)/3 = 24/3 = 8 % 5 = 3
        let copy = MODULO;
        println!("{}", copy);
    }
}

// expected output
// 3
