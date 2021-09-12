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

With this crate, the macros derive [`plctag::GetValue`] and [`plctag::SetValue`] for you automatically.

### Examples

```rust
use plctag_core::RawTag;
use plctag_derive::{GetValue, SetValue};

#[derive(Debug, Default, GetValue, SetValue)]
struct MyUDT {
    #[offset(0)]
    a: u32,
    #[offset(4)]
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
