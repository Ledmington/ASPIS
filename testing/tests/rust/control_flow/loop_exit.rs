// Validate correct handling of loop control flow and basic block transitions.
//
// Rust port of ../../c/control_flow/loop_exit.c: `#[unsafe(link_section = "aspis_to_harden")]`
// stands in for clang's `__attribute__((annotate("to_harden")))`, translated by the
// rust-annotation-bridge pass into the same @llvm.global.annotations entry. #![no_std] / #![no_main]
// keep this freestanding so the emitted IR links with plain `clang` at the end of the ASPIS
// pipeline, with no Rust std runtime involved.
#![no_std]
#![no_main]

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
}

#[unsafe(link_section = "aspis_to_harden")]
#[unsafe(no_mangle)]
pub static mut sum: i32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
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
            sum += i;
        }
        i += 1;
    }
    unsafe {
        printf(b"%d\0".as_ptr(), sum);
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// expected output
// 2
