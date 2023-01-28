use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plctag::futures::AsyncTag;
use std::{cell::RefCell, sync::Arc};

fn bench_read(c: &mut Criterion) {
    c.bench_function("async read", |b| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
            let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.0.83&name=Car_Pos[1]&elem_count=1";
        let tag = AsyncTag::new(path).unwrap();
        let entry = Arc::new(RefCell::new(tag));
        b.to_async(rt).iter_batched(
            || entry.clone(),
            |entry| async move {
                let tag = &mut* entry.borrow_mut();
                let _value: i32 = tag.read_value(0).await.unwrap();
            },
            BatchSize::PerIteration,
        )
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
