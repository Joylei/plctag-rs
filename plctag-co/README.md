# plctag-co

coroutine wrapper based on [plctag-rs](../plctag).

## How to use

Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.

Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.

Add `plctag` to your Cargo.toml

```toml
[dependencies]
plctag= { git="https://github.com/Joylei/plctag-rs.git", path="plctag-co"}
may="*"
```

You're OK to build your project.

## Examples

 ```rust
use plctag_async::{TagEntry, TagFactory, TagOptions, GetValue, SetValue};
use std::fmt;

struct MyTagOptions {
    pub key: String,
    pub path: String,
}

impl TagOptions for MyTagOptions {
    fn key(&self)->&str{
        &self.key
    }
}

impl fmt::Display for MyTagOptions{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}
let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION

let factory = TagFactory::new();
let opts = MyTagOptions {
    key: String::from("192.168.1.120;MyTag1"),
    path: path.to_owned(),
};
let tag = factory.create(opts);
let connected = tag.connect(Some(Duration::from_millis(150)));
assert!(connected);
let offset = 0;
let value:u16 = tag.read_value(offset).unwrap();
println!("tag value: {}", value);

let value = value + 10;
tag.write_value(offset, value).unwrap();
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
