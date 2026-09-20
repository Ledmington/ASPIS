//! Marker wrapper types for ASPIS's Rust front-end support.
//!
//! Clang tells ASPIS which globals/functions to selectively harden,
//! duplicate, or exclude via `__attribute__((annotate("...")))`, which
//! lowers to an `llvm.global.annotations` entry that ASPIS's passes parse
//! directly (see `passes/Utils/Utils.cpp`, `getFuncAnnotations`). rustc has
//! no equivalent attribute, so a `static`'s marking has to be recovered
//! from somewhere else entirely: give it the type `ToHarden<X>` (etc.) and
//! ASPIS's `rust-annotation-bridge` pass recognizes it by that type name
//! and converts it into the same `llvm.global.annotations` entry clang
//! would have produced, so nothing downstream of it needs to change:
//!
//! ```ignore
//! static mut COUNTER: ToHarden<i32> = ToHarden::new(0);
//! ```
//!
//! | wrapper type    | annotation     |
//! |-----------------|----------------|
//! | `ToHarden<X>`   | `to_harden`    |
//! | `ToDuplicate<X>`| `to_duplicate` |
//! | `Exclude<X>`    | `exclude`      |
//!
//! **This only works when compiled with debug info (`-g`).** rustc's own
//! codegen already lowers these single-field wrapper structs down to `X`'s
//! raw storage layout regardless of `repr` - which is convenient in that
//! there's no unwrapping to do, but it also means the wrapper's identity
//! leaves no trace in the *plain* LLVM IR type system. The only place
//! "this static's Rust type was `ToHarden<i32>`" survives at all is DWARF
//! debug info. Without `-g`, the bridge pass has nothing to find and the
//! annotation is silently dropped - same failure mode as ASPIS's own C-side
//! parser hitting IR that doesn't match clang's expected shape.
//!
//! The wrapper types above only apply to data, since there's no way to wrap
//! a function item in a generic type. Functions are marked directly with
//! `#[link_section]` instead - functions are `GlobalValue`s in LLVM IR just
//! like statics, and a section name survives to IR independent of debug
//! info, so this path doesn't share the above limitation:
//!
//! ```ignore
//! // main is invoked indirectly through std::rt::lang_start, not by a
//! // direct call ASPIS could rewrite, so it must stay untouched; harden
//! // the real logic in a plain function instead.
//! #[link_section = "aspis_exclude"]
//! fn main() {
//!     aspis_main();
//! }
//!
//! #[no_mangle]
//! extern "C" fn aspis_main() { /* ... */ }
//! ```

use std::ops::{AddAssign, Deref, DerefMut};

macro_rules! annotated_wrapper {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        pub struct $name<X>(pub X);

        impl<X> $name<X> {
            pub const fn new(value: X) -> Self {
                Self(value)
            }
        }

        impl<X> Deref for $name<X> {
            type Target = X;
            #[inline(always)]
            fn deref(&self) -> &X {
                &self.0
            }
        }

        impl<X> DerefMut for $name<X> {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut X {
                &mut self.0
            }
        }

        impl<X, Rhs> AddAssign<Rhs> for $name<X>
        where
            X: AddAssign<Rhs>,
        {
            #[inline(always)]
            fn add_assign(&mut self, rhs: Rhs) {
                self.0 += rhs;
            }
        }
    };
}

annotated_wrapper!(
    /// Requires the module to be compiled with debug info (`-g`). Only
    /// affects ASPIS's selective-checking techniques (REDDI); a no-op
    /// under the default `DUPLICATE_ALL` techniques (EDDI/SEDDI/FDSC),
    /// which harden everything regardless of annotation.
    ToHarden
);

annotated_wrapper!(
    /// Requires the module to be compiled with debug info (`-g`). Marks
    /// the global for ASPIS's `DuplicateGlobals` pass.
    ToDuplicate
);

annotated_wrapper!(
    /// Requires the module to be compiled with debug info (`-g`).
    /// Excludes the global from ASPIS hardening entirely.
    Exclude
);
