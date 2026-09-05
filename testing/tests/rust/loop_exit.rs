// Validate correct handling of loop control flow and basic block transitions.
// Rust equivalent of ../c/control_flow/loop_exit.c, used as the first probe
// for whether ASPIS's LLVM-IR passes tolerate rustc-generated IR at all.
//
// SUM mirrors loop_exit.c's `__attribute__((annotate("to_harden"))) int
// sum` global: wrapped in ToHarden<i32>, which ASPIS's rust-annotation-bridge
// pass recognizes by name (via debug info - this file must be compiled with
// -g, see rust-annotations/src/lib.rs) and converts into the same
// llvm.global.annotations entry clang emits for the C attribute.
//
// #![no_main]: by default rustc auto-generates an unmangled C-ABI `main`
// that calls std::rt::lang_start, which in turn indirectly calls the
// user's `fn main()` through a function-pointer argument. That
// auto-generated `main` can't be annotated (we don't write it), so ASPIS's
// EDDI duplicates it like any other function - and its runtime-init IR
// shapes (scalar-pair returns, calling-convention allocas around argc/argv)
// confuse ASPIS's TypeDeductionAnalysis enough to corrupt argv's type,
// crashing the verifier at the lang_start call. Writing the C-ABI `main`
// ourselves (bypassing lang_start entirely) gives us a single real entry
// point we can mark `exclude` directly, exactly like a C test's `main`.

#![no_main]

use aspis_annotations::ToHarden;

static mut SUM: ToHarden<i32> = ToHarden::new(0);

// static_mut_refs fires on any reference to a mutable static, sound or not;
// harmless here since this is a single-threaded test fixture.
#[allow(static_mut_refs)]
#[no_mangle]
extern "C" fn aspis_main() {
    for i in 0..5 {
        if i == 1 {
            continue;
        }
        if i == 3 {
            break;
        }
        unsafe {
            SUM += i;
        }
    }
    unsafe {
        print!("{}", *SUM);
    }
}

#[link_section = "aspis_exclude"]
#[no_mangle]
extern "C" fn main() -> i32 {
    aspis_main();
    // lang_start normally flushes stdout on exit; bypassing it via
    // #![no_main] means we have to do that ourselves.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    0
}

// expected output
// 2
