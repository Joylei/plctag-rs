use plctag_core::RawTag;
use plctag_derive::{GetValue, SetValue};

#[derive(Debug, Default, GetValue, SetValue)]
struct MyUDT {
    #[offset(0)]
    a: u32,
    #[offset(4)]
    b: u32,
}

#[test]
fn test_derive() {
    let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();
    let res = tag.read(100);
    assert!(res.is_ok());
    let udt: MyUDT = tag.get_value(0).unwrap();
    assert_eq!(udt.a, 4);
    assert_eq!(udt.b, 0);
}
