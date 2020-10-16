use criterion::{black_box, criterion_group, criterion_main, Criterion};
use space_indexer::{bvh::test::bvh_build, utils::generate_boxes_in_space};

fn criterion_benchmark(c: &mut Criterion) {
  let boxes = generate_boxes_in_space(black_box(20000), black_box(1000.), black_box(1.));
  c.bench_function("bvh build perf", |b| b.iter(|| bvh_build(&boxes)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
