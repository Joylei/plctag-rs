use criterion::{black_box, criterion_group, criterion_main, Criterion};
use plctag::{RawTag, ValueExt};

fn bench_read(c: &mut Criterion) {
    let timeout = 1000; //ms
    let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.0.83&name=Car_Pos[1]&elem_count=1";
    let tag = RawTag::new(path, timeout).unwrap();

    let read_tag = move || {
        let status = tag.read(timeout);
        if !status.is_ok() {
            panic!("failed to read");
        }
        let value: i32 = tag.get_value(0).unwrap();
        value
    };
    c.bench_function("sync read", |b| {
        b.iter(|| {
            // Inner closure, the actual test
            black_box(read_tag());
        })
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
