use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plctag::futures::{AsyncTag, Pool};
use tokio::task;

fn bench_read(c: &mut Criterion) {
    c.bench_function("async batch-20 read", |b| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let pool = rt.block_on(async {
            Pool::new()
        });
        let tags:Vec<_> = (0..20).map(|i| format!("protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.0.83&name=Car_Pos[{}]&elem_count=1", i)).collect();

        b.to_async(rt).iter_batched(||  (pool.clone(), tags.clone()) ,
            |(pool, tags)| async move {
                let tasks =  tags.iter().map(|path|{
                    let pool = pool.clone();
                    let path = path.clone();
                    task::spawn(async move {
                        let tag = pool.entry(path).await.unwrap();
                        let tag_ref = tag.get().await.unwrap();
                        let _value: i32 = tag_ref.read_value(0).await.unwrap();
                    })
                   });
                   futures::future::join_all(tasks).await;
            }, BatchSize::PerIteration
        )
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
