// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-rs

a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.

[![crates.io](https://img.shields.io/crates/v/plctag.svg)](https://crates.io/crates/plctag)
[![docs](https://docs.rs/plctag/badge.svg)](https://docs.rs/plctag)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## How to use

Add `plctag` to your Cargo.toml

```toml
[dependencies]
plctag= "0.3"
```

## crates

- [plctag](https://crates.io/crates/plctag) reexports everything from below crates.
- [plctag-core](https://crates.io/crates/plctag-core) a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.
- [plctag-async](https://crates.io/crates/plctag-async) async wrapper.
- [plctag-log](https://crates.io/crates/plctag-log) log adapter for `libplctag`
- [plctag-derive](https://crates.io/crates/plctag-derive) macros for `plctag`
- [plctag-sys](https://crates.io/crates/plctag-sys) native libplctag binding

## Examples

### read/write tag

```rust,no_run
use plctag::{Encode, Decode, RawTag, ValueExt};
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

### UDT

read/write UDT

```rust,no_run
use plctag::{Decode, Encode, RawTag, Result, ValueExt};

// define your UDT
#[derive(Default, Debug, Decode, Encode)]
struct MyUDT {
    #[tag(offset = 0)]
    v1: u16,
    #[tag(offset = 2)]
    v2: u16,
}

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
```

Note:
Do not perform expensive operations when you derives `Decode` or `Encode`.

### Async

```rust,no_run
use plctag::futures::{Error, AsyncTag};
use tokio::runtime;

let rt = runtime::Runtime::new().unwrap();
let res: Result<_, Error> = rt.block_on(async {
    let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION
    let mut tag = AsyncTag::create(path).await?;
    let offset = 0;
    let value: u16 = tag.read_value(offset).await?;
    println!("tag value: {}", value);

    let value = value + 10;
    tag.write_value(offset, value).await?;
    Ok(())
});
res.unwrap();

```

### Path Builder

```rust,no_run
use plctag::builder::*;
use plctag::RawTag;

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

```

### Logging adapter for `libplctag`

```rust,no_run
use plctag::log::log_adapt;
use plctag::log::set_debug_level;
use plctag::log::DebugLevel;

log_adapt(); //register logger
set_debug_level(DebugLevel::Info); // set debug level

// now, you can receive log messages by any of logging implementations of crate `log`

```

## Build

Please refer to [How to build](https://github.com/Joylei/plctag-rs/tree/master/crates/sys#build) to setup build environment.

## License

MIT

*/

#[doc(inline)]
pub use plctag_core::*;
#[cfg(feature = "derive")]
#[doc(inline)]
pub use plctag_derive::{Decode, Encode};
#[cfg(feature = "log")]
#[doc(inline)]
pub use plctag_log as log;

#[cfg(feature = "async")]
#[doc(inline)]
pub use plctag_async as futures;

#[cfg(feature = "async")]
pub use plctag_async::AsyncTag;
