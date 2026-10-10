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

struct Pair {
    a: i32,
    b: i32,
}

// Helper to print two pointer values (non-duplicated)
#[unsafe(link_section = "aspis_to_harden")]
fn print_pointers(p1: &i32, p2: Option<&i32>) {
    print!("Value pointed by p1: {}", p1);
    if let Some(p2) = p2 {
        print!(", Value pointed by p2: {}", p2);
    }
    println!();
}

fn main() {
    // Allocate a small array on the heap
    let buffer: Box<[i32; 2]> = Box::new([100, 200]);
    // Use a single pointer + offset instead of two aliases
    let base = buffer.as_ptr();
    let _second_value = unsafe { *base.add(1) };

    // Example struct with two distinct heap allocations
    let obj = Box::new(Pair { a: 1, b: 2 });
    let _sum = obj.a + obj.b;

    // Example of copying data instead of aliasing
    let p1 = Box::new(42);
    let p2 = Box::new(*p1); // copy the value

    // Print the results (non-duplicated)
    print_pointers(&p1, Some(&p2));
}
