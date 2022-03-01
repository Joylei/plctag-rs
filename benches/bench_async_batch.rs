use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plctag::futures::AsyncTag;
use std::sync::Arc;
use tokio::{sync::Mutex, task};

fn bench_read(c: &mut Criterion) {
    c.bench_function("async batch-20 read", |b| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let tags = rt.block_on(async {
            let mut tags = vec![];
            for i in 0..20 {
                let options =  format!("protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.0.83&name=Car_Pos[{}]&elem_count=1", i);
                let tag = AsyncTag::create(options).await.unwrap();
                tags.push(Arc::new(Mutex::new(tag)));
            };
            tags
        });


        b.to_async(rt).iter_batched(||  tags.clone(),
            |tags| async move {
                let tasks =  tags.into_iter().map(|tag|{
                    task::spawn(async move {
                        let mut guard = tag.lock().await;
                        let _value: i32 = guard.read_value(0).await.unwrap();
                    })
                   });
                   futures::future::join_all(tasks).await;
            }, BatchSize::PerIteration
        )
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
