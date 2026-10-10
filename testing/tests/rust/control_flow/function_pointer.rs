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

fn foo() -> i32 {
    42
}

#[unsafe(link_section = "aspis_to_harden")]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    // Simple function pointer call
    let fptr: fn() -> i32 = foo;
    let result = fptr();
    println!("{}", result);

    // Function pointer call with parameters
    let addptr: fn(i32, i32) -> i32 = add;
    let sum = addptr(27, result);
    println!("{}", sum);
}
