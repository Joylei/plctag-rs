// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use plctag_core::{RawTag, ValueExt};
use plctag_derive::{Decode, Encode};

#[derive(Debug, Default, Decode, Encode)]
struct MyUDT {
    #[tag(offset = 0)]
    a: u32,
    #[tag(offset = 4)]
    b: u32,
}

#[derive(Debug, Default, Decode, Encode)]
struct MyUDT2 {
    #[tag(offset = 0)]
    a: u32,
    #[tag(encode_fn = "my_encode", decode_fn = "my_decode")]
    b: u32,
}

fn my_encode(v: &u32, tag: &RawTag, offset: u32) -> plctag_core::Result<()> {
    tag.set_u32(offset + 4, *v)
}

fn my_decode(tag: &RawTag, offset: u32) -> plctag_core::Result<u32> {
    tag.get_u32(offset + 4).map(|v| v + 1)
}

#[test]
fn test_derive() {
    let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();
    let res = tag.read(100);
    assert!(res.is_ok());
    let udt: MyUDT = tag.get_value(0).unwrap();
    assert_eq!(udt.a, 4);
    assert_eq!(udt.b, 0);
    let udt2: MyUDT2 = tag.get_value(0).unwrap();
    assert_eq!(udt2.a, 4);
    assert_eq!(udt2.b, 0);
}
