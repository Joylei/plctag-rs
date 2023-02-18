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
plctag-core= "0.4"
```

## No-std support

The `plctag-core` crate has a Cargo feature named "std" that is enabled by default. In order to use the crate in a no_std context this feature needs to be disabled.

```toml
[dependencies]
plctag-core = { version = "0.4", default-features = false }
```

## Examples

### read/write tag

```rust
use plctag_core::{Encode, Decode, RawTag, ValueExt};
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


## Build

Please refer to [How to build](https://github.com/Joylei/plctag-rs/tree/master/crates/sys#build) to setup build environment.

## License

MIT
