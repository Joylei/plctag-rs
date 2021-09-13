// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-derive

macros for plctag-rs

## Usage

please use it with [crate@plctag]

With this crate, the macros derive [`plctag::Decode`] and [`plctag::Encode`] for you automatically.

### Examples

```rust,ignore
use plctag_core::RawTag;
use plctag_derive::{Decode, Encode};

#[derive(Debug, Default, Decode, Encode)]
struct MyUDT {
    #[tag(offset=0)]
    a: u32,
    #[tag(offset=4)]
    b: u32,
}


fn main() {
    let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();
    let res = tag.read(100);
    assert!(res.is_ok());
    let udt: MyUDT = tag.get_value(0).unwrap();
    assert_eq!(udt.a, 4);
    assert_eq!(udt.b, 0);
}

```

## License

MIT

*/

extern crate proc_macro;

mod decode_derive;
mod encode_derive;
mod shared;

use proc_macro::TokenStream;
use syn::DeriveInput;

use syn::parse_macro_input;

#[proc_macro_derive(Decode, attributes(tag))]
pub fn get_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    decode_derive::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Encode, attributes(tag))]
pub fn set_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    encode_derive::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
