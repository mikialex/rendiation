//! The shader trim queries against the CPU f32 mirror of `TrimEncoding` (plan-trim T1).

use parametric_normalized_model::*;
use parametric_tessellation::test_scenes::trim::*;
use parametric_tessellation::test_scenes::*;
use parametric_tessellation::trim::*;

use super::*;
use crate::*;

fn scene_encoding(accel: &dyn TrimAccel) -> TrimEncoding {
  let model = ParametricNormalizedModel {
    patches: trim_scenes(),
    ..Default::default()
  };
  encode_trims(&model, &trim_config(), accel)
}

/// Runs `kernel` once per case of `stride` input words, the kernel writes one u32.
async fn run_trim_kernel(
  gpu: &GPU,
  name: &'static str,
  trim: &TrimDeviceData,
  inputs: &[Vec4<u32>],
  stride: u32,
  kernel: impl FnOnce(&ShaderTrimData, &dyn Fn(u32) -> Node<Vec4<u32>>) -> Node<u32>,
) -> Vec<u32> {
  let cases = inputs.len() as u32 / stride;
  let input = create_gpu_readonly_storage(inputs, gpu, "trim kernel inputs");
  let out = create_gpu_read_write_storage::<[u32]>(
    ZeroedArrayByArrayLength(cases as usize),
    gpu,
    "trim kernel out",
  );
  let hasher = PipelineHasher::default().with_hash(name);
  let pipeline = gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(64);
      let shader_trim = trim.bind_shader(builder.bindgroups());
      let input = builder.bind_by(&input);
      let out = builder.bind_by(&out);
      let case = builder.global_invocation_id().x();
      if_by(case.greater_equal_than(val(cases)), do_return);
      let read = |k: u32| input.index(case * val(stride) + val(k)).load();
      let result = kernel(&shader_trim, &read);
      out.index(case).store(result);
      builder
    });
  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    let mut binding = BindingBuilder::default();
    trim.bind_pass(&mut binding);
    binding
      .with_bind(&input)
      .with_bind(&out)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(cases.div_ceil(64), 1, 1);
  });
  let result = encoder.read_storage_array(&gpu.device, &out);
  gpu.submit_encoder(encoder);
  result.await.unwrap()
}

fn as_uv(v: Vec4<u32>) -> Vec2<f32> {
  Vec2::new(f32::from_bits(v.y), f32::from_bits(v.z))
}

/// Shortest distance from `uv` (clamped) to the segments of the patch, dense sampling in f64.
fn segment_distance(encoding: &TrimEncoding, patch: usize, uv: Vec2<f32>) -> f64 {
  let Some(view) = encoding.patch_view(patch) else {
    return f64::INFINITY;
  };
  let p = clamp_uv(uv).into_f64();
  let mut distance = f64::INFINITY;
  for s in view.segments {
    let (p0, p1, p2) = (s.p0.into_f64(), s.p1.into_f64(), s.p2.into_f64());
    for i in 0..=1024 {
      let t = i as f64 / 1024.;
      let q = p0 * ((1. - t) * (1. - t)) + p1 * (2. * t * (1. - t)) + p2 * (t * t);
      distance = distance.min((q - p).length());
    }
  }
  distance
}

/// The points of `sample_points` per scene patch: random and on every critical line.
fn point_cases(encoding: &TrimEncoding, patches: usize) -> Vec<Vec4<u32>> {
  let mut rng = TestRng(17);
  let mut cases = Vec::new();
  for patch in 0..patches {
    for p in sample_points(encoding, patch, &mut rng, 6000) {
      cases.push(Vec4::new(patch as u32, p.x.to_bits(), p.y.to_bits(), 0));
    }
  }
  cases
}

#[pollster::test]
async fn trim_winding_matches_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  for accel in [&BandGrid::DEFAULT as &dyn TrimAccel, &AllSegments] {
    let encoding = scene_encoding(accel);
    let trim = TrimDeviceData::new(&gpu.device, &encoding);
    let cases = point_cases(&encoding, trim_scenes().len());
    let result = run_trim_kernel(&gpu, "trim winding", &trim, &cases, 1, |trim, read| {
      let c = read(0);
      let uv: Node<Vec2<f32>> = (c.y().bitcast::<f32>(), c.z().bitcast::<f32>()).into();
      trim.winding(c.x(), uv).bitcast::<u32>()
    })
    .await;
    let (mut mismatched, mut max_mismatch_distance) = (0, 0f64);
    for (case, &gpu_winding) in cases.iter().zip(&result) {
      let (patch, uv) = (case.x as usize, as_uv(*case));
      let cpu = encoding.winding(patch, uv);
      if cpu != gpu_winding as i32 {
        mismatched += 1;
        let distance = segment_distance(&encoding, patch, uv);
        max_mismatch_distance = max_mismatch_distance.max(distance);
      }
    }
    println!(
      "kind {}: {} points, {mismatched} mismatched, max mismatch distance {max_mismatch_distance:e}",
      accel.kind(),
      cases.len()
    );
    // Only samples on a segment may differ: at a segment end point the root's x is rounding
    // noise, whose sign depends on the FMA contraction and the division precision.
    assert!(max_mismatch_distance < 1e-6, "{max_mismatch_distance}");
  }
}

#[pollster::test]
async fn trim_rect_class_matches_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut rng = TestRng(19);
  for accel in [&BandGrid::DEFAULT as &dyn TrimAccel, &AllSegments] {
    let encoding = scene_encoding(accel);
    let trim = TrimDeviceData::new(&gpu.device, &encoding);
    let mut cases = Vec::new();
    let mut rects = Vec::new();
    for patch in 0..trim_scenes().len() as u32 {
      for _ in 0..3000 {
        let level = (rng.unit() * 8.) as u32;
        let n = (1u32 << level) as f32;
        let (ix, iy) = (
          (rng.unit() * n as f64) as u32,
          (rng.unit() * n as f64) as u32,
        );
        let rect = TrimRect {
          min: Vec2::new(ix as f32 / n, iy as f32 / n),
          max: Vec2::new((ix + 1) as f32 / n, (iy + 1) as f32 / n),
        };
        cases.push(Vec4::new(patch, 0, 0, 0));
        cases.push(Vec4::new(
          rect.min.x.to_bits(),
          rect.min.y.to_bits(),
          rect.max.x.to_bits(),
          rect.max.y.to_bits(),
        ));
        rects.push((patch as usize, rect));
      }
    }
    let result = run_trim_kernel(&gpu, "trim rect", &trim, &cases, 2, |trim, read| {
      let r = read(1).bitcast::<Vec4<f32>>();
      trim.classify_rect(read(0).x(), r.xy(), r.zw())
    })
    .await;
    let mut classes = [0; 3];
    for ((patch, rect), &gpu_class) in rects.iter().zip(&result) {
      let cpu = rect_class_code(encoding.classify_rect(*patch, rect));
      assert_eq!(cpu, gpu_class, "patch {patch} {rect:?}");
      classes[cpu as usize] += 1;
    }
    println!("kind {}: rect classes {classes:?}", accel.kind());
    assert!(classes.iter().all(|&n| n > 100), "{classes:?}");
  }
}

/// The shader queries against the CPU mirror on every trimmed patch of the steam controller
/// model (skipped when the file is missing): `cargo test --release -p
/// rendiation-parametric-rendering steam_trim -- --ignored --nocapture`.
#[pollster::test]
#[ignore]
async fn steam_trim_queries_match_cpu() {
  let Ok(model) = ParametricNormalizedModel::load_pnm(super::tests_frame::STEAM_MODEL) else {
    println!("steam model not found, skipped");
    return;
  };
  let gpu = GPU::new(Default::default()).await.unwrap();
  let encoding = encode_model_trims(&model);
  let trim = TrimDeviceData::new(&gpu.device, &encoding);
  let mut rng = TestRng(23);
  let mut points = Vec::new();
  let mut rects = Vec::new();
  for patch in 0..model.patches.len() {
    if encoding.patch_view(patch).is_none() {
      continue;
    }
    for p in sample_points(&encoding, patch, &mut rng, 24) {
      points.push(Vec4::new(patch as u32, p.x.to_bits(), p.y.to_bits(), 0));
    }
    for _ in 0..8 {
      let level = (rng.unit() * 6.) as u32;
      let n = (1u32 << level) as f32;
      let (ix, iy) = (
        (rng.unit() * n as f64) as u32,
        (rng.unit() * n as f64) as u32,
      );
      let min = Vec2::new(ix as f32 / n, iy as f32 / n);
      let max = Vec2::new((ix + 1) as f32 / n, (iy + 1) as f32 / n);
      rects.push(Vec4::new(patch as u32, 0, 0, 0));
      rects.push(Vec4::new(
        min.x.to_bits(),
        min.y.to_bits(),
        max.x.to_bits(),
        max.y.to_bits(),
      ));
    }
  }
  let windings = run_trim_kernel(&gpu, "trim winding", &trim, &points, 1, |trim, read| {
    let c = read(0);
    let uv: Node<Vec2<f32>> = (c.y().bitcast::<f32>(), c.z().bitcast::<f32>()).into();
    trim.winding(c.x(), uv).bitcast::<u32>()
  })
  .await;
  let (mut mismatched, mut max_distance) = (0, 0f64);
  for (case, &w) in points.iter().zip(&windings) {
    let (patch, uv) = (case.x as usize, as_uv(*case));
    if encoding.winding(patch, uv) != w as i32 {
      mismatched += 1;
      max_distance = max_distance.max(segment_distance(&encoding, patch, uv));
    }
  }
  let classes = run_trim_kernel(&gpu, "trim rect", &trim, &rects, 2, |trim, read| {
    let r = read(1).bitcast::<Vec4<f32>>();
    trim.classify_rect(read(0).x(), r.xy(), r.zw())
  })
  .await;
  let mut rect_mismatched = 0;
  for (case, &class) in rects.chunks(2).zip(&classes) {
    let r = case[1];
    let rect = TrimRect {
      min: Vec2::new(f32::from_bits(r.x), f32::from_bits(r.y)),
      max: Vec2::new(f32::from_bits(r.z), f32::from_bits(r.w)),
    };
    let cpu = rect_class_code(encoding.classify_rect(case[0].x as usize, &rect));
    rect_mismatched += (cpu != class) as usize;
  }
  println!(
    "{} points, {mismatched} mismatched (max distance to a segment {max_distance:e}), {} rects, \
     {rect_mismatched} mismatched",
    points.len(),
    classes.len()
  );
  assert!(max_distance < 1e-6 && rect_mismatched == 0);
}
