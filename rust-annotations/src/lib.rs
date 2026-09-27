use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Item;

/// Annotates a function or a static with an ASPIS annotation, e.g. `#[aspis(to_harden)]`.
///
/// Alongside the item, this emits a marker static placed in the `aspis.annotations` section:
/// a `{ ptr to item, ptr to "annotation\0" }` pair. The RustAnnotationBridge LLVM pass turns
/// every marker into an entry of `llvm.global.annotations`, the same form clang emits for
/// `__attribute__((annotate(...)))`, and then deletes the markers.
#[proc_macro_attribute]
pub fn aspis(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed: Item = match syn::parse(item.clone()) {
        Ok(item) => item,
        Err(e) => return e.to_compile_error().into(),
    };

    let (symbol, target) = match &parsed {
        Item::Static(item) => {
            let ident = &item.ident;
            (ident.clone(), quote! { &raw const #ident as *const () })
        }
        Item::Fn(item) => {
            if !item.sig.generics.params.is_empty() {
                return syn::Error::new_spanned(
                    &item.sig.generics,
                    "#[aspis] does not support generic functions",
                )
                .to_compile_error()
                .into();
            }
            let ident = &item.sig.ident;
            (ident.clone(), quote! { #ident as *const () })
        }
        _ => {
            return syn::Error::new_spanned(parsed, "#[aspis] only supports functions and statics")
                .to_compile_error()
                .into();
        }
    };

    let annotation = match syn::parse::<syn::Ident>(attr) {
        Ok(ident) => ident.to_string(),
        Err(e) => return e.to_compile_error().into(),
    };

    let marker_type = format_ident!("__AspisMarker_{}_{}", annotation, symbol);
    let marker_name = format_ident!("__ASPIS_{}_{}", annotation, symbol);
    let item: proc_macro2::TokenStream = item.into();

    quote! {
        #item

        #[allow(non_camel_case_types)]
        #[repr(C)]
        struct #marker_type(*const (), *const u8);
        unsafe impl Sync for #marker_type {}

        #[used]
        #[unsafe(link_section = "aspis.annotations")]
        static #marker_name: #marker_type =
            #marker_type(#target, concat!(#annotation, "\0").as_ptr());
    }
    .into()
}
