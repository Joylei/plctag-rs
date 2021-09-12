extern crate proc_macro;

mod get_value;
mod set_value;
mod shared;

use proc_macro::TokenStream;
use syn::DeriveInput;

use syn::parse_macro_input;

#[proc_macro_derive(GetValue, attributes(offset))]
pub fn get_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    get_value::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(SetValue, attributes(offset))]
pub fn set_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    set_value::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
