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
static mut KEY: u8 = 0x5A;

fn xor_crypt(data: u8, k: u8) -> u8 {
    data ^ k
}

#[unsafe(link_section = "aspis_to_harden")]
fn process_buffer(buf: &mut [u8], k: u8) {
    for byte in buf.iter_mut() {
        *byte = xor_crypt(*byte, k);
    }
}

fn main() {
    let original: [u8; 10] = *b"HELLOWORLD";
    let mut buffer: [u8; 10] = original;

    let key = unsafe { KEY };
    process_buffer(&mut buffer, key); // Encrypt
    process_buffer(&mut buffer, key); // Decrypt

    if buffer == original {
        println!("SUCCESS");
    } else {
        println!("FAIL");
    }
}

// expected output
// SUCCESS
