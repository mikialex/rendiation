use criterion::{black_box, criterion_group, criterion_main, Criterion};
use space_indexer::{
  bvh::test::bvh_build, bvh::BalanceTree, bvh::SAH, utils::generate_boxes_in_space,
  utils::TreeBuildOption,
};

fn criterion_benchmark(c: &mut Criterion) {
  let boxes = generate_boxes_in_space(black_box(20000), black_box(10000.), black_box(1.));

  // c.bench_function("cross at algebra", |b| {
  //   use rendiation_algebra::*;
  //   let va: Vector3<f32> = black_box(Vector3::from([1., 3., 4.]));
  //   let vb: Vector3<f32> = black_box(Vector3::from([1., 1., 4.]));
  //   b.iter(|| va.cross(vb))
  // });
  // c.bench_function("cross at math", |b| {
  //   use rendiation_math::*;
  //   let va: Vec3<f32> = black_box(Vec3::new(1., 3., 4.));
  //   let vb: Vec3<f32> = black_box(Vec3::new(1., 1., 4.));
  //   b.iter(|| va.cross(vb))
  // });

  c.bench_function("mat inverse at algebra", |b| {
    use rendiation_algebra::*;
    let m: Matrix3<f32> = black_box(Matrix3::one());
    b.iter(|| m.inverse().unwrap())
  });
  c.bench_function("mat inverse at math", |b| {
    use rendiation_math::*;
    let m: Mat3<f32> = black_box(Mat3::one());
    b.iter(|| m.inverse().unwrap())
  });

  // c.bench_function("balance bvh build perf", |b| {
  //   b.iter(|| {
  //     bvh_build(
  //       &boxes,
  //       &mut BalanceTree,
  //       &TreeBuildOption {
  //         max_tree_depth: 15,
  //         bin_size: 10,
  //       },
  //     )
  //   })
  // });

  // c.bench_function("sah bvh build perf", |b| {
  //   b.iter(|| {
  //     bvh_build(
  //       &boxes,
  //       &mut SAH::new(4),
  //       &TreeBuildOption {
  //         max_tree_depth: 15,
  //         bin_size: 10,
  //       },
  //     )
  //   })
  // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
