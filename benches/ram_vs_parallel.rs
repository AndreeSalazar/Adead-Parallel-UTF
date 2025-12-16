use criterion::{black_box, criterion_group, criterion_main, Criterion};
use adead_parallel_utf::{Resolver, UtfId};
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use tempfile::tempdir;

fn generate_data(count: usize, len: usize) -> Vec<String> {
    let mut rng = thread_rng();
    (0..count)
        .map(|_| {
            (&mut rng)
                .sample_iter(Alphanumeric)
                .take(len)
                .map(char::from)
                .collect()
        })
        .collect()
}

fn bench_ram_vs_ssd(c: &mut Criterion) {
    let data_count = 1000; // Smaller for quick bench
    let string_len = 100;
    let data = generate_data(data_count, string_len);

    let mut group = c.benchmark_group("storage");

    // 1. RAM Write (Vec Push)
    group.bench_function("ram_write", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(data_count);
            for s in &data {
                vec.push(s.clone());
            }
            black_box(vec);
        })
    });

    // 2. SSD Write (Register)
    // We use a new temp dir per iter is too slow.
    // We'll reuse one resolver and generate unique data per iter?
    // Or just measure the cost of checking + writing unique items?
    // Let's just measure "register" on a fresh batch each time? No.
    // We'll just measure the overhead of the mechanism on a single large batch.
    let dir = tempdir().unwrap();
    let path = dir.path().join("bench.puf");
    let resolver = Resolver::new(&path).unwrap();
    
    // To measure write properly, we'd need to clear the file.
    // For this bench, we'll accept that repeated runs in criterion might hit the index cache.
    // But let's try to feed unique data.
    
    group.bench_function("puf_register_idempotent", |b| {
        b.iter(|| {
             for s in &data {
                 black_box(resolver.register_utf(s).unwrap());
             }
        })
    });

    // 3. RAM Read
    let vec = data.clone();
    group.bench_function("ram_read", |b| {
        b.iter(|| {
            for s in &vec {
                black_box(s);
            }
        })
    });

    // 4. SSD Read (Resolve)
    let ids: Vec<UtfId> = data.iter().map(|s| resolver.register_utf(s).unwrap()).collect();
    group.bench_function("puf_resolve", |b| {
        b.iter(|| {
            for &id in &ids {
                let r = resolver.resolve_utf(id).unwrap();
                black_box(&*r);
            }
        })
    });
    
    group.finish();
}

criterion_group!(benches, bench_ram_vs_ssd);
criterion_main!(benches);
