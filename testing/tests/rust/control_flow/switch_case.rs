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

fn switch_test(value: i32) -> i32 {
    match value {
        0 => 100,
        1 => 200,
        2 => 250,
        3 => 300,
        4 => 400,
        _ => -1,
    }
}

#[unsafe(link_section = "aspis_to_harden")]
static mut SWITCH_N: i32 = 3;

fn main() {
    let result = switch_test(unsafe { SWITCH_N });
    println!("{}", result);
}

// expected output
// 300
