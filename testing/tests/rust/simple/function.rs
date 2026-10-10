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
fn foo() -> i32 {
    let c = 12;
    let d = 13;
    c + d
}

fn main() -> std::process::ExitCode {
    let a = 10;
    let b = 20;
    let c = foo();
    println!("foo() {}", c);
    std::process::ExitCode::from(if a > b { 1 } else { 0 })
}
