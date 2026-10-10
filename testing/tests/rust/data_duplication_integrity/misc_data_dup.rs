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
static mut DUPLICATED_GLOBAL: i32 = 100;

#[unsafe(link_section = "aspis_exclude")]
static mut EXCLUDED_GLOBAL: i32 = 200;

// Arrays are not auto-duplicated by DuplicateGlobals unless explicitly marked
// "to_duplicate" (unlike plain scalar globals, which always are).
#[unsafe(link_section = "aspis_to_duplicate")]
static mut LOOKUP: [i32; 4] = [1, 2, 3, 4];

fn increment(x: i32) -> i32 {
    x + 1
}

#[unsafe(link_section = "aspis_to_harden")]
fn multiply_by_two(x: i32) -> i32 {
    x * 2
}

#[unsafe(link_section = "aspis_exclude")]
fn secret_func(x: i32) -> i32 {
    x - 42
}

fn main() {
    unsafe {
        let mut val = DUPLICATED_GLOBAL;
        val = increment(val);
        val = multiply_by_two(val);

        let mut excl = EXCLUDED_GLOBAL;
        excl += 5;

        let secret = secret_func(excl);

        let extra = LOOKUP[0] + LOOKUP[3];

        let result = val + excl + secret + extra;

        if result == 575 {
            println!("OK");
        } else {
            println!("FAIL");
        }
    }
}

// expected output
// OK
