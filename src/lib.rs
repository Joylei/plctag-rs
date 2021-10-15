// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
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
plctag= "0.2"
```

## crates

- [plctag](https://crates.io/crates/plctag) reexports everything from below crates.
- [plctag-core](https://crates.io/crates/plctag-core) a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.
- [plctag-async](https://crates.io/crates/plctag-async) tokio based async wrapper.
- [plctag-log](https://crates.io/crates/plctag-log) log adapter for `libplctag`
- [plctag-derive](https://crates.io/crates/plctag-derive) macros for `plctag`
- [plctag-sys](https://crates.io/crates/plctag-sys) native libplctag binding

## Examples

### read/write tag

```rust,ignore
use plctag::{Encode, Decode, RawTag};
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

```rust,ignore
use plctag::{Decode, Encode, RawTag, Result};

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

    value.v1 = value.v1 + 10;
    tag.set_value(offset, value).unwrap();

    //write tag
    let status = tag.write(timeout);
    assert!(status.is_ok());
    println!("write done!");
}
```

Note:
Do not perform expensive operations when you derives `Decode` or `Encode`.

### Async

```rust,ignore
use plctag::futures::{AsyncTag, Error, TagEntry};

use tokio::runtime;

fn main() {
    let rt = runtime::Runtime::new().unwrap();
    let res: Result<_, Error> = rt.block_on(async {
        let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION
        let tag = TagEntry::create(path).await?;
        let tag_ref = tag.get().await?;
        let offset = 0;
        let value: u16 = tag_ref.read_value(offset).await?;
        println!("tag value: {}", value);

        let value = value + 10;
        tag_ref.write_value(offset, value).await?;
        Ok(())
    });
    res.unwrap();
}

```

### Path Builder

```rust,ignore
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

```

### Logging adapter for `libplctag`

```rust,ignore
use plctag::log::log_adapt;
use plctag::log::set_debug_level;
use plctag::log::DebugLevel;

log_adapt(); //register logger
set_debug_level(DebugLevel::Info); // set debug level

// now, you can receive log messages by any of logging implementations of crate `log`

```

## Thread-safety

Operations are not thread-safe in this library except async wrappers, please use `std::sync::Mutex` or something similar to enforce thread-safety.

## Build & Test

Please refer to `How to use` to setup build environment.

Because mutithread will cause troubles, you need to run tests with:

```shell
cargo test --all -- --test-threads=1
```

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
