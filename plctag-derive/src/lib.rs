extern crate proc_macro;

mod tag_derive;

use proc_macro::TokenStream;
use syn::DeriveInput;

use syn::parse_macro_input;
use tag_derive::expand_tag_derive;

#[proc_macro_derive(TagValue)]
pub fn tag_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
