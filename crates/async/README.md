# plctag-async

async wrapper based on [plctag-rs](../plctag).

## How to use

Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.

Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.

Add `plctag` to your Cargo.toml

```toml
[dependencies]
plctag= { git="https://github.com/Joylei/plctag-rs.git", path="plctag-async"}
```

You're OK to build your project.

## Examples

```rust
use plctag_async::{TagEntry, TagFactory, TagOptions, Decode, Encode};
use tokio::runtime;
use std::fmt;



let rt = runtime::Runtime::new().unwrap()?;
rt.block_on(async {
   let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION

   let tag = TagEntry::create(path).await;
   let tag_ref = tag.get().await;
   let offset = 0;
   let value:u16 = tag_ref.read_value(offset).await.unwrap();
   println!("tag value: {}", value);

   let value = value + 10;
   tag_ref.write_value(offset).await.unwrap();
});
```

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
