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
    0
}

fn main() {
    let sasso_carta = 1;
    let mut filippo_congenito = 2;
    let mut i = foo();
    while i < sasso_carta + filippo_congenito {
        filippo_congenito -= 1;
        i += 1;
    }
    println!("{}", filippo_congenito);
}
