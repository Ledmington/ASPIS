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
static mut RES: f32 = 0.0;

fn main() {
    let i: i32 = 5;
    let f: f32 = 2.5;
    let c: i8 = 3;
    let l: i64 = 4;

    unsafe {
        // 5 + 2.5 + 3 + 4 = 14.5
        RES = i as f32 + f + c as f32 + l as f32;
        let copy = RES;
        println!("{:.1}", copy);
    }
}

// expected output
// 14.5
