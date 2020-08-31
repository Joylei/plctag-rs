use plctag::{Accessor, GetValue, RawTag, Result, SetValue, TagValue};

// define your UDT
#[derive(Default, Debug)]
struct MyUDT {
    v1: u16,
    v2: u16,
}
impl TagValue for MyUDT {
    fn get_value(&mut self, accessor: &dyn Accessor, offset: u32) -> Result<()> {
        self.v1.get_value(accessor, offset)?;
        self.v2.get_value(accessor, offset + 2)?;
        Ok(())
    }

    fn set_value(&self, accessor: &dyn Accessor, offset: u32) -> Result<()> {
        self.v1.set_value(accessor, offset)?;
        self.v2.set_value(accessor, offset + 2)?;
        Ok(())
    }
}

fn main() {
    let timeout = 100; //ms
    let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16"; // YOUR TAG DEFINITION
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
