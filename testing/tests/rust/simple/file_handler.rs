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

// Simulates a resource (like a file) using RAII: `Drop` stands in for the C++
// destructor. No annotation here (matches the original), since this test is about
// ASPIS not breaking scope-based cleanup, not about hardening a particular value.
struct FakeFileHandler;

impl FakeFileHandler {
    fn new() -> Self {
        println!("Handler created");
        FakeFileHandler
    }
}

impl Drop for FakeFileHandler {
    fn drop(&mut self) {
        println!("File closed");
    }
}

fn main() {
    let _handler = FakeFileHandler::new();
}
