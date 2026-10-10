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

// Print function (non-duplicated)
#[unsafe(link_section = "aspis_exclude")]
fn print_result(value: i32) {
    println!("Result: {}", value);
}

// Example of a function template
fn my_max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// Example of a simple accumulator struct template
struct Accumulator<T> {
    sum: T,
}

impl<T: std::ops::AddAssign> Accumulator<T> {
    fn add(&mut self, value: T) {
        self.sum += value; // duplicated safely
    }
}

impl<T: Copy> Accumulator<T> {
    fn total(&self) -> T {
        self.sum
    }
}

#[unsafe(link_section = "aspis_to_harden")]
static mut ACC: Accumulator<i32> = Accumulator { sum: 0 };

fn main() {
    let x = 42;
    let y = 17;
    let max_val = my_max(x, y); // 42

    let acc = unsafe { &mut *(&raw mut ACC) };
    acc.add(5);
    acc.add(10);
    let sum_val = acc.total(); // 15

    print_result(max_val);
    print_result(sum_val);
}
