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
fn print_result(value: i32) {
    println!("{}", value);
}

#[inline]
fn square(x: i32) -> i32 {
    x * x
}

const fn get_five() -> i32 {
    5
}

fn main() {
    let a = 3;
    let b = 4;

    let result1 = square(a); // 9
    let result2 = square(b); // 16
    const C: i32 = get_five(); // 5

    print_result(result1);
    print_result(result2);
    print_result(C);
}
