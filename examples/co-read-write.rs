use plctag_async::{TagEntry, TagFactory, TagOptions, TagValue};
struct MyTagOptions {
    pub key: String,
    pub path: String,
}

impl TagOptions for MyTagOptions {
    fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for MyTagOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

fn main() {
    let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION

    let factory = TagFactory::new();
    let opts = MyTagOptions {
        key: String::from("192.168.1.120;MyTag1"),
        path: path.to_owned(),
    };
    let tag = factory.create(opts);
    let connected = tag.connect(Some(Duration::from_millis(150)));
    assert!(connected);
    let offset = 0;
    let value: u16 = tag.read_value(offset).unwrap();
    println!("tag value: {}", value);

    let value = value + 10;
    tag.write_value(offset, value).unwrap();
}
