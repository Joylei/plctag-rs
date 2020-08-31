# plctag-rs

a rust wrapper of [`libplctag`](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.

## Features

- synchronous APIs
- asynchronous APIs based on Tokio
- tag path builder
- UDT support

## Examples

### read/write tag

 ```rust
 use plctag::{RawTag, TagValue, GetValue, SetValue};
 let timeout = 100;//ms
 let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION
 let tag = RawTag::new(path, timeout).unwrap();

 //read tag
 let status = tag.read(timeout);
 assert!(status.is_ok());
 let offset = 0;
 let value:u16 = tag.get_value(offset).unwrap();
 println!("tag value: {}", value);

 let value = value + 10;
 tag.set_value(offset, value).unwrap();

 //write tag
 let status = tag.write(timeout);
 assert!(status.is_ok());
 println!("write done!");
 ```

### async read/write tag

 ```rust
 use plctag::future::AsyncTag;
 use tokio::runtime::Runtime;

 let mut rt = Runtime::new()::unwrap();
 rt.block_on(async move {
     // YOUR TAG DEFINITION
     let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";
     let tag = AsyncTag::new(path).await.unwrap();

     let offset = 0;
     let value:u16 = 100;
     //write tag
     tag.set_and_write(offset, value).await.unwrap();
     // read tag
     let value:u16 = tag.read_and_get(offset).await.unwrap();
     assert_eq!(value, 100);
 });

 ```

### UDT

read/write UDT

 ```rust
 use plctag::{Accessor, TagValue, RawTag, GetValue, SetValue, Result};

 // define your UDT
 #[derive(Default, Debug)]
 struct MyUDT {
     v1:u16,
     v2:u16,
 }
 impl TagValue for MyUDT {
     fn get_value(&mut self, accessor: &dyn Accessor, offset: u32) -> Result<()>{
         self.v1.get_value(accessor, offset)?;
         self.v2.get_value(accessor, offset + 2)?;
         Ok(())
     }

     fn set_value(&self, accessor: &dyn Accessor, offset: u32) -> Result<()>{
         self.v1.set_value(accessor, offset)?;
         self.v1.set_value(accessor, offset + 2)?;
         Ok(())
     }
 }

 fn main(){
     let timeout = 100;//ms
     // YOUR TAG DEFINITION
     let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16";
     let tag = RawTag::new(path, timeout).unwrap();

     //read tag
     let status = tag.read(timeout);
     assert!(status.is_ok());
     let offset = 0;
     let mut value:MyUDT = tag.get_value(offset).unwrap();
     println!("tag value: {:?}", value);

     value.v1 = value.v1 + 10;
     tag.set_value(offset, value).unwrap();

     //write tag
     let status = tag.write(timeout);
     assert!(status.is_ok());
     println!("write done!");
 }

 ```

## Thread-safety

 Operations in `libplctag` are guarded with mutex, so they are somewhat thread safe.
 But imagine that one thread sets a value for a tag, another thread can set a different value for the same
 tag once it acquires the mutex lock before the previous thread perform other operations on the tag.
 It is that you still need some sync mechanism to make sure your sequence of operations
 are atomic.

## Test

Because mutithread will cause troubles, you need to run tests with:

```shell
cargo test -- --test-threads=1
```
