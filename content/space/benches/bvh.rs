use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rendiation_space_algorithm::{
  bvh::BalanceTree, bvh::SAH, bvh::bvh_build, utils::TreeBuildOption,
  utils::generate_boxes_in_space,
};

fn criterion_benchmark(c: &mut Criterion) {
  let boxes = generate_boxes_in_space(black_box(20000), black_box(10000.), black_box(1.));

  c.bench_function("balance bvh build perf", |b| {
    b.iter(|| {
      bvh_build(
        &boxes,
        &mut BalanceTree,
        &TreeBuildOption {
          max_tree_depth: 15,
          bin_size: 10,
        },
      )
    })
  });

  c.bench_function("sah bvh build perf", |b| {
    b.iter(|| {
      bvh_build(
        &boxes,
        &mut SAH::new(4),
        &TreeBuildOption {
          max_tree_depth: 15,
          bin_size: 10,
        },
      )
    })
  });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
