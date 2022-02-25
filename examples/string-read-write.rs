// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use plctag::{Decode, Encode, RawTag, Result, ValueExt};
use std::{
    cmp,
    ops::{Deref, DerefMut},
};

/// string tag: S4, capacity 4
#[derive(Debug, Default)]
struct S4(String);

impl Deref for S4 {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for S4 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Encode for S4 {
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        const CAPACITY: u32 = 4;
        let bytes = self.0.as_bytes();
        let count = cmp::min(CAPACITY, bytes.len() as u32);
        //DINT
        let old_count = tag.get_i32(offset)? as u32;
        tag.set_value(offset, count)?;
        let pos = offset + 4;
        // N of SINT
        if count > 0 {
            tag.set_bytes_unchecked(pos, &bytes[0..count as usize])?;
        }
        //remaining, clear to 0
        if old_count > count {
            let diff = old_count - count;
            let bytes = vec![0; diff as usize];
            let pos = pos + count;
            tag.set_bytes_unchecked(pos, &bytes)?;
        }
        Ok(())
    }
}

impl Decode for S4 {
    fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
        let mut res: Self = Default::default();
        let pos = offset;
        //DINT
        let count = tag.get_i32(pos)? as u32;
        if count == 0 {
            //skip read
            res.0.truncate(0);
        } else {
            let pos = offset + 4;
            // N of SINT
            let mut bytes = vec![0; count as usize];
            tag.get_bytes_unchecked(pos, &mut bytes)?;
            res.0 = unsafe { String::from_utf8_unchecked(bytes) };
        }
        Ok(res)
    }
}

fn main() {
    let timeout = 100; //ms
                       // YOUR TAG DEFINITION
    let path =
        "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1";
    let tag = RawTag::new(path, timeout).unwrap();

    //read tag
    let status = tag.read(timeout);
    assert!(status.is_ok());
    let offset = 0;
    let mut value: S4 = tag.get_value(offset).unwrap();
    println!("tag value: {:?}", value);

    *value = "cdef".to_owned();
    tag.set_value(offset, value).unwrap();

    //write tag
    let status = tag.write(timeout);
    assert!(status.is_ok());
    println!("write done!");
}
