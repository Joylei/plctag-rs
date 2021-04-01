use plctag::{GetValue, RawTag, Result, SetValue};

// define your UDT
#[derive(Default, Debug)]
struct MyUDT {
    v1: u16,
    v2: u16,
}
impl GetValue for MyUDT {
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()> {
        self.v1.get_value(tag, offset)?;
        self.v2.get_value(tag, offset + 2)?;
        Ok(())
    }
}
impl SetValue for MyUDT {
    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
        self.v1.set_value(tag, offset)?;
        self.v2.set_value(tag, offset + 2)?;
        Ok(())
    }
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
