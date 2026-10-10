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

#[unsafe(link_section = "aspis_exclude")]
fn run_no_dup<F: FnMut()>(func: &mut F) {
    func();
}

#[unsafe(link_section = "aspis_to_harden")]
fn run<F: FnMut()>(func: &mut F) {
    func();
}

fn main() {
    // Example 1: closure capturing a local variable by reference
    let mut x: i32 = 0;
    let mut incr_x = |val: i32| {
        x += val;
        println!("x incremented by {}", val);
    };
    incr_x(5); // can be duplicated safely (x is local to each duplicate)

    // Example 2: closure capturing heap-allocated memory
    let mut p = Box::new(10);
    let mut inc_ptr = || {
        *p += 1;
    };
    run_no_dup(&mut inc_ptr); // non-duplicated increment of shared memory
    run(&mut inc_ptr); // duplicated increment of shared memory

    println!("Value pointed by p: {}", p);
}
