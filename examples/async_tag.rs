use plctag::future::AsyncTag;
use tokio::runtime::Runtime;

fn main() {
    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16"; // YOUR TAG DEFINITION
        let tag = AsyncTag::new(path).await.unwrap();
        let offset = 0;
        let value: u16 = 100;
        //write tag
        tag.set_and_write(offset, value).await.unwrap();
        // read tag
        let value: u16 = tag.read_and_get(offset).await.unwrap();
        assert_eq!(value, 100);
    });
}
