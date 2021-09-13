# plctag-rs

a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.

## How to use

Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.

Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.

Add `plctag` to your Cargo.toml

```toml
[dependencies]
plctag= { git="https://github.com/Joylei/plctag-rs.git", path="plctag"}
```

You're OK to build your project.

```shell
cargo build
```

## Examples

### read/write tag

```rust
use plctag::{Accessor, RawTag};
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
use plctag::{RawTag, Result, Decode, Encode};

// define your UDT
#[derive(Default, Debug)]
struct MyUDT {
    v1:u16,
    v2:u16,
}
impl Decode for MyUDT {
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>{
        self.v1.get_value(tag, offset)?;
        self.v2.get_value(tag, offset + 2)?;
        Ok(())
    }
}
 impl Encode for MyUDT {
    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()>{
        self.v1.set_value(tag, offset)?;
        self.v2.set_value(tag, offset + 2)?;
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

Note:
Do not perform expensive operations when you derives `Decode` or `Encode`.

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
use plctag::logging::log_adapt;
use plctag::plc::set_debug_level;
use plctag::DebugLevel;

log_adapt(); //register logger
set_debug_level(DebugLevel::Info); // set debug level

// now, you can receive log messages by any of logging implementations of crate `log`

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
