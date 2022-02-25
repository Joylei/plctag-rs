// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-core

a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.

[![crates.io](https://img.shields.io/crates/v/plctag-core.svg)](https://crates.io/crates/plctag-core)
[![docs](https://docs.rs/plctag-core/badge.svg)](https://docs.rs/plctag-core)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## How to use

Add `plctag-core` to your Cargo.toml

```toml
[dependencies]
plctag-core= "0.1"
```

## Examples

### read/write tag

```rust,ignore
use plctag_core::{Encode, Decode, RawTag};
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

### More examples

please take a look at [examples](../../examples/)

## Thread-safety

Operations are not thread-safe in this library, please use `std::sync::Mutex` or something similar to enforce thread-safety.

## Build & Test

Please refer to `How to use` to setup build environment.

Because mutithread will cause troubles, you need to run tests with:

```shell
cargo test -- --test-threads=1
```

## License

MIT

*/
#![warn(missing_docs)]

extern crate plctag_sys;

/// reexports ffi Apis
pub mod ffi {
    pub use plctag_sys::*;
}

pub mod builder;
mod debug;
mod raw;
mod status;
#[cfg(feature = "value")]
mod value;

/// plctag result
pub type Result<T> = std::result::Result<T, Status>;
pub use raw::RawTag;
pub use status::Status;

#[cfg(feature = "value")]
pub use value::{Decode, Encode, ValueExt};
