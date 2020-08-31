use plctag::builder::*;

fn main() {
    let timeout = 100;
    let res = TagBuilder::new()
        .config(|builder| {
            builder
                .protocol(Protocol::EIP)
                .gateway("192.168.1.120")
                .plc(PlcKind::ControlLogix)
                .name("MyTag1")
                .element_size(16)
                .element_count(1)
                .path("1,0")
                .read_cache_ms(0);
        })
        .create(timeout);
    assert!(res.is_ok());
    let tag = res.unwrap();
    let status = tag.status();
    assert!(status.is_ok());
}
