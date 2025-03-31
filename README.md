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
plctag= "0.4"
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

```rust
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

```rust
use plctag::{Decode, Encode, RawTag, Result};

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

```rust
use plctag::futures::{AsyncTag, Error};

use tokio::runtime;

let rt = runtime::Runtime::new().unwrap();
let res: Result<_, Error> = rt.block_on(async {
    let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION
    let tag = AsyncTag::create(path).await?;
    let tag_ref = tag.get().await?;
    let offset = 0;
    let value: u16 = tag_ref.read_value(offset).await?;
    println!("tag value: {}", value);

    let value = value + 10;
    tag_ref.write_value(offset, value).await?;
    Ok(())
});
res.unwrap();

```

### Path Builder

```rust
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

```rust
use plctag::log::log_adapt;
use plctag::log::set_debug_level;
use plctag::log::DebugLevel;

log_adapt(); //register logger
set_debug_level(DebugLevel::Info); // set debug level

// now, you can receive log messages by any of logging implementations of crate `log`

```

## Build

Please refer to [How to build](https://github.com/Joylei/plctag-rs/tree/master/crates/sys#build) to setup build environment.


### Static build
Please refer to [Static build](https://github.com/Joylei/plctag-rs/tree/master/crates/sys#Static%20build)


## Bench

```shell
cargo bench
```

The plots and saved data are stored under target/criterion/$BENCHMARK_NAME/

### Bench Result

Processor: Intel(R) Core(TM) i7-9700KF CPU @ 3.60GHz   3.60 GHz
RAM: 64.0 GB

```
async read              time:   [4.3608 ms 4.3937 ms 4.4287 ms]
                        change: [-3.7035% -2.5545% -1.3730%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 7 outliers among 100 measurements (7.00%)
  7 (7.00%) high mild

async batch-20 read     time:   [11.852 ms 11.949 ms 12.054 ms]
                        change: [-2.2194% -0.9951% +0.2445%] (p = 0.11 > 0.05)
                        No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) low mild
  3 (3.00%) high mild
  1 (1.00%) high severe

sync read               time:   [3.1016 ms 3.1272 ms 3.1553 ms]
                        change: [-2.4462% -1.1947% +0.1535%] (p = 0.07 > 0.05)
                        No change in performance detected.
Found 9 outliers among 100 measurements (9.00%)
  6 (6.00%) high mild
  3 (3.00%) high severe
```

## License

MIT
