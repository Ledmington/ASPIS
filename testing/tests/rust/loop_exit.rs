// Needed to compile a source file without a Cargo.toml
extern crate aspis_rust_annotations;

use aspis_rust_annotations::aspis;

use std::io::Write;

#[aspis(to_harden)]
pub static mut SUM: i32 = 0;

fn DataCorruption_Handler() {
    println!("ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler");
    std::io::stdout().flush().unwrap();
}

fn SigMismatch_Handler() {
    println!("ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler");
    std::io::stdout().flush().unwrap();
}

fn main() {
    let mut i = 0;

    while i < 5 {
        if i == 1 {
            i += 1;
            continue;
        }
        if i == 3 {
            break;
        }
        unsafe {
            SUM += i;
        }
        i += 1;
    }

    unsafe {
        println!("{}", SUM);
    }
}

// expected output
// 2
