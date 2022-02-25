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

```rust
use plctag::futures::{AsyncTag, Error};

use tokio::runtime;

fn main() {
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
}

```

### Path Builder

```rust
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

## Bench

```shell
cargo bench
```

The plots and saved data are stored under target/criterion/$BENCHMARK_NAME/

### Bench Result

Processor: Intel(R) Core(TM) i7-9700KF CPU @ 3.60GHz   3.60 GHz
RAM: 64.0 GB

```
sync read               time:   [5.8721 ms 5.9099 ms 5.9585 ms]
                        change: [+0.0881% +0.7138% +1.4994%] (p = 0.06 > 0.05)
                        No change in performance detected.
Found 8 outliers among 100 measurements (8.00%)
  2 (2.00%) low severe
  1 (1.00%) low mild

async read              time:   [8.5650 ms 8.6148 ms 8.6786 ms]
                        change: [-1.1842% -0.2853% +0.7350%] (p = 0.55 > 0.05)
                        No change in performance detected.
Found 7 outliers among 100 measurements (7.00%)
  1 (1.00%) low severe


async batch-20 read     time:   [18.205 ms 18.431 ms 18.676 ms]
                        change: [-2.0615% -0.1733% +1.5987%] (p = 0.86 > 0.05)
                        No change in performance detected.
Found 3 outliers among 100 measurements (3.00%)
  2 (2.00%) high mild
  1 (1.00%) high severe
```

## License

MIT
