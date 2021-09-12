// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use plctag::builder::*;
use plctag::RawTag;

fn main() {
    let timeout = 100;
    let path = PathBuilder::default()
        .protocol(Protocol::EIP)
        .gateway("192.168.1.120")
        .plc(PlcKind::ControlLogix)
        .name("MyTag1")
        .element_size(16)
        .element_count(1)
        .path("1,0")
        .read_cache_ms(0)
        .build()
        .unwrap();
    let tag = RawTag::new(path, timeout).unwrap();
    let status = tag.status();
    assert!(status.is_ok());
}
