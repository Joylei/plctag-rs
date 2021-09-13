use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plctag::futures::{AsyncTag, TagEntry};

fn bench_read(c: &mut Criterion) {
    c.bench_function("async read 10", |b| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let tag = rt.block_on(async {
            let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.0.83&name=Car_Pos[1]&elem_count=1";
            let tag = TagEntry::create(path).await.unwrap();
            tag
        });
        b.to_async(rt).iter_batched(
            || tag.clone(),
            |tag| async move {
                for _i in 0..10 {
                    let tag_ref = tag.get().await.unwrap();
                    let _value: i32 = tag_ref.read_value(0).await.unwrap();
                    //value
                }
            },
            BatchSize::PerIteration,
        )
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
