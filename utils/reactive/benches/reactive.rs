use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reactive::do_updates;

/// this mainly measure the overhead of channel
fn criterion_benchmark(c: &mut Criterion) {
  let (send, mut rev) = black_box(futures::channel::mpsc::unbounded::<u32>());

  c.bench_function("check do update on empty channel receiver stream", |b| {
    b.iter(|| {
      do_updates(&mut rev, |v| {
        black_box(v);
      })
    })
  });

  c.bench_function(
    "do update on channel receiver stream, one value send, one value consume",
    |b| {
      b.iter(|| {
        send.unbounded_send(1).unwrap();
        do_updates(&mut rev, |v| {
          black_box(v);
        })
      })
    },
  );

  c.bench_function(
    "do update on channel receiver stream, many value send, many value consume",
    |b| {
      b.iter(|| {
        for _ in 0..black_box(100) {
          send.unbounded_send(1).unwrap();
        }
        do_updates(&mut rev, |v| {
          black_box(v);
        })
      })
    },
  );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
