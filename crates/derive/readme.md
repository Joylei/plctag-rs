# plctag-derive

macros for plctag-rs

## Usage

please use it with [crate@plctag]

With this crate, the macros derive [`plctag::Decode`] and [`plctag::Encode`] for you automatically.

### Examples

```rust
use plctag_core::RawTag;
use plctag_derive::{Decode, Encode};

#[derive(Debug, Default, Decode, Encode)]
struct MyUDT {
    #[tag(offset=0)]
    a: u32,
    #[tag(offset=4)]
    b: u32,
}


fn main() {
    let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();
    let res = tag.read(100);
    assert!(res.is_ok());
    let udt: MyUDT = tag.get_value(0).unwrap();
    assert_eq!(udt.a, 4);
    assert_eq!(udt.b, 0);
}

```

## License

MIT
