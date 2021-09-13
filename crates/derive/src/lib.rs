// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

extern crate proc_macro;

mod decode_derive;
mod encode_derive;
mod shared;

use proc_macro::TokenStream;
use syn::DeriveInput;

use syn::parse_macro_input;

/// the macro derives `plctag_core::Decode` for you automatically.
///
/// ```rust,ignore
/// use plctag_core::RawTag;
/// use plctag_derive::{Decode, Encode};
///
/// #[derive(Debug, Default, Decode)]
/// struct MyUDT {
///    #[tag(offset=0)]
///    a: u32,
///    #[tag(offset=4)]
///    b: u32,
/// }
/// ```
#[proc_macro_derive(Decode, attributes(tag))]
pub fn decode_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    decode_derive::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// the macro derives `plctag_core::Encode` for you automatically.
///
/// ```rust,ignore
/// use plctag_core::RawTag;
/// use plctag_derive::{Decode, Encode};
///
/// #[derive(Debug, Default, Encode)]
/// struct MyUDT {
///    #[tag(offset=0)]
///    a: u32,
///    #[tag(offset=4)]
///    b: u32,
/// }
/// ```
#[proc_macro_derive(Encode, attributes(tag))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    encode_derive::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
