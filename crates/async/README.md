# plctag-async

tokio based async wrapper for `libplctag`.

[![crates.io](https://img.shields.io/crates/v/plctag-async.svg)](https://crates.io/crates/plctag-async)
[![docs](https://docs.rs/plctag-async/badge.svg)](https://docs.rs/plctag-async)
[![build](https://github.com/joylei/plctag-rs/workflows/Test%20and%20Build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22Test+and+Build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## How to use

Add `plctag-async` to your Cargo.toml

```toml
[dependencies]
plctag-async= "0.1"
```

## Examples

```rust,ignore
use plctag_async::{AsyncTag, Error, TagEntry};
use tokio::runtime;

let rt = runtime::Runtime::new().unwrap()?;
rt.block_on(async {
   let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION

   let tag = TagEntry::create(path).await.unwrap();
   let tag_ref = tag.get().await.unwrap();
   let offset = 0;
   let value:u16 = tag_ref.read_value(offset).await.unwrap();
   println!("tag value: {}", value);

   let value = value + 10;
   tag_ref.write_value(offset, value).await.unwrap();
});
```

## Thread-safety

It's thread-safe to perform operations with `plctag-async`.

## Build & Test

Please refer to `How to use` to setup build environment.

Because mutithread will cause troubles, you need to run tests with:

```shell
cargo test -- --test-threads=1
```

## License

MIT
