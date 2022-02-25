// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use plctag::{Decode, Encode, RawTag, Result, ValueExt};

// define your UDT
#[derive(Default, Debug, Decode, Encode)]
struct MyUDT {
    #[tag(offset = 0)]
    v1: u16,
    #[tag(offset = 2)]
    v2: u16,
}

fn main() {
    let timeout = 100; //ms
                       // YOUR TAG DEFINITION
    let path = "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16";
    let tag = RawTag::new(path, timeout).unwrap();

    //read tag
    let status = tag.read(timeout);
    assert!(status.is_ok());
    let offset = 0;
    let mut value: MyUDT = tag.get_value(offset).unwrap();
    println!("tag value: {:?}", value);

    value.v1 += 10;
    tag.set_value(offset, value).unwrap();

    //write tag
    let status = tag.write(timeout);
    assert!(status.is_ok());
    println!("write done!");
}
