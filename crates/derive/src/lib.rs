// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-derive

macros for `plctag`

[![crates.io](https://img.shields.io/crates/v/plctag-derive.svg)](https://crates.io/crates/plctag-derive)
[![docs](https://docs.rs/plctag-derive/badge.svg)](https://docs.rs/plctag-derive)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## Usage

please use it with [plctag](https://crates.io/crates/plctag)

With this crate, the macros derive `plctag_core::Decode` and `plctag_core::Encode` for you automatically.

### Examples

```rust,no_run
use plctag_core::{RawTag, Result, ValueExt};
use plctag_derive::{Decode, Encode};

#[derive(Debug, Default, Decode, Encode)]
struct MyUDT {
    #[tag(offset=0)]
    a: u32,
    #[tag(offset=4)]
    b: u32,
    #[tag(decode_fn="my_decode", encode_fn="my_encode")]
    c: u32,
 }

fn my_decode(tag:&RawTag, offset: u32)->plctag::Result<u32> {
    tag.get_u32(offset + 8).map(|v|v+1)
}

fn my_encode(v: &u32, tag: &RawTag, offset: u32)->plctag::Result<()> {
    tag.set_u32(offset + 8, *v - 1)
}

let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();
let res = tag.read(100);
assert!(res.is_ok());
let udt: MyUDT = tag.get_value(0).unwrap();
assert_eq!(udt.a, 4);
assert_eq!(udt.b, 0);

```

## License

MIT

*/
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
/// ```rust,no_run
/// use plctag_core::RawTag;
/// use plctag_derive::{Decode, Encode};
///
/// #[derive(Debug, Default, Decode)]
/// struct MyUDT {
///    #[tag(offset=0)]
///    a: u32,
///    #[tag(offset=4)]
///    b: u32,
///    #[tag(decode_fn="my_decode")]
///    c: u32,
/// }
///
/// fn my_decode(tag:&RawTag, offset: u32)->plctag::Result<u32> {
///     tag.get_u32(offset + 8).map(|v|v+1)
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
/// ```rust,no_run
/// use plctag_core::RawTag;
/// use plctag_derive::{Decode, Encode};
///
/// #[derive(Debug, Default, Encode)]
/// struct MyUDT {
///    #[tag(offset=0)]
///    a: u32,
///    #[tag(offset=4)]
///    b: u32,
///    #[tag(encode_fn="my_encode")]
///    c: u32,
/// }
///
/// fn my_encode(v: &u32, tag: &RawTag, offset: u32)->plctag::Result<()> {
///     tag.set_u32(offset + 8, *v - 1)
/// }
/// ```
#[proc_macro_derive(Encode, attributes(tag))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    encode_derive::expand_tag_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
