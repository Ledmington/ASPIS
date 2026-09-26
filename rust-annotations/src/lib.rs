use proc_macro::TokenStream;
use quote::quote;
use syn::Item;

#[proc_macro_attribute]
pub fn aspis(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr = {}", attr);
    println!("item = {}", item);

    let parsed: Item = match syn::parse(item.clone()) {
        Ok(item) => item,
        Err(e) => return e.to_compile_error().into(),
    };

    let symbol_name = match &parsed {
        Item::Static(item) => item.ident.to_string(),
        Item::Fn(item) => item.sig.ident.to_string(),
        Item::Const(item) => item.ident.to_string(),
        Item::Struct(item) => item.ident.to_string(),
        Item::Enum(item) => item.ident.to_string(),
        Item::Type(item) => item.ident.to_string(),
        Item::Mod(item) => item.ident.to_string(),
        _ => {
            return syn::Error::new_spanned(parsed, "#[aspis] does not support this item")
                .to_compile_error()
                .into();
        }
    };

    let attr_name = attr.to_string();

    let marker_name = syn::Ident::new(
        &format!("__ASPIS_{}_{}", attr_name, symbol_name),
        proc_macro2::Span::call_site(),
    );

    let item: proc_macro2::TokenStream = item.into();

    quote! {
        #item

        #[used]
        #[unsafe(no_mangle)]
        pub static #marker_name: &[u8] =
            concat!(#attr_name, "\0").as_bytes();
    }
    .into()
}
