// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use plctag::futures::{Error, TagEntry};

use tokio::runtime;

fn main() {
    let rt = runtime::Runtime::new().unwrap();
    let res: Result<_, Error> = rt.block_on(async {
        let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION
        let mut tag = TagEntry::create(path).await?;
        let offset = 0;
        let value: u16 = tag.read_value(offset).await?;
        println!("tag value: {}", value);

        let value = value + 10;
        tag.write_value(offset, value).await?;
        Ok(())
    });
    res.unwrap();
}
