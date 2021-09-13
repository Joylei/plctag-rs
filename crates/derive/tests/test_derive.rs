// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use plctag_core::RawTag;
use plctag_derive::{Decode, Encode};

#[derive(Debug, Default, Decode, Encode)]
struct MyUDT {
    #[tag(offset = 0)]
    a: u32,
    #[tag(offset = 4)]
    b: u32,
}

#[test]
fn test_derive() {
    let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();
    let res = tag.read(100);
    assert!(res.is_ok());
    let udt: MyUDT = tag.get_value(0).unwrap();
    assert_eq!(udt.a, 4);
    assert_eq!(udt.b, 0);
}
