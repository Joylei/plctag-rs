// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use plctag::RawTag;

fn main() {
    let timeout = 100; //ms
    let path = "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION
    let tag = RawTag::new(path, timeout).unwrap();
    //read tag
    let status = tag.read(timeout);
    assert!(status.is_ok());
    let offset = 0;
    let value: u16 = tag.get_value(offset).unwrap();
    println!("tag value: {}", value);
    let value = value + 10;
    tag.set_value(offset, value).unwrap();
    //write tag
    let status = tag.write(timeout);
    assert!(status.is_ok());
    println!("write done!");
    // tag will be destroyed when out of scope or manually call drop()
    drop(tag);
}
