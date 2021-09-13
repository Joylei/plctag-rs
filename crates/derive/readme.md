# plctag-derive

macros for `plctag`

[![crates.io](https://img.shields.io/crates/v/plctag-derive.svg)](https://crates.io/crates/plctag-derive)
[![docs](https://docs.rs/plctag-derive/badge.svg)](https://docs.rs/plctag-derive)
[![build](https://github.com/joylei/plctag-rs/workflows/Test%20and%20Build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22Test+and+Build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## Usage

please use it with [plctag](https://crates.io/crates/plctag)

With this crate, the macros derive `plctag_core::Decode` and `plctag_core::Encode` for you automatically.

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
