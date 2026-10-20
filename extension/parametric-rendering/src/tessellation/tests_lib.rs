//! The shader functions against the f64 CPU reference on random inputs (plan M2). Every test
//! kernel runs one case per thread, reading control points and integer parameters from storage
//! and writing `Vec4<u32>` results (floats bitcast).

use parametric_normalized_model::*;
use parametric_tessellation::test_scenes::*;
use parametric_tessellation::*;

use super::*;
use crate::*;

struct KernelInput {
  cps: Vec<Vec4<f32>>,
  params: Vec<Vec4<u32>>,
  param_stride: u32,
  out_stride: u32,
}

impl KernelInput {
  fn cases(&self) -> u32 {
    self.params.len() as u32 / self.param_stride
  }
}

struct KernelCtx {
  u: ENode<TessUniform>,
  cps: ShaderReadonlyPtrOf<[Vec4<f32>]>,
  params: ShaderReadonlyPtrOf<[Vec4<u32>]>,
  out: ShaderPtrOf<[Vec4<u32>]>,
  case: Node<u32>,
  param_stride: u32,
  out_stride: u32,
}

impl KernelCtx {
  fn param(&self, k: u32) -> Node<Vec4<u32>> {
    self
      .params
      .index(self.case * val(self.param_stride) + val(k))
      .load()
  }

  fn write_u(&self, k: impl Into<Node<u32>>, v: Node<Vec4<u32>>) {
    let index = self.case * val(self.out_stride) + k.into();
    self.out.index(index).store(v);
  }

  fn write_f(&self, k: impl Into<Node<u32>>, v: Node<Vec4<f32>>) {
    self.write_u(k, v.bitcast::<Vec4<u32>>());
  }

  fn read_f(&self, k: Node<u32>) -> Node<Vec4<f32>> {
    let index = self.case * val(self.out_stride) + k;
    self.out.index(index).load().bitcast::<Vec4<f32>>()
  }
}

fn vec3_to_4(v: Node<Vec3<f32>>, w: Node<f32>) -> Node<Vec4<f32>> {
  fvec4(v.x(), v.y(), v.z(), w)
}

async fn run_kernel(
  gpu: &GPU,
  name: &'static str,
  input: &KernelInput,
  uniform: TessUniform,
  kernel: impl FnOnce(&KernelCtx),
) -> Vec<Vec4<u32>> {
  let cps = create_gpu_readonly_storage(input.cps.as_slice(), gpu, "cps");
  let params = create_gpu_readonly_storage(input.params.as_slice(), gpu, "params");
  let uniform = create_uniform(uniform, gpu, "uniform");
  let cases = input.cases();
  let out = create_gpu_read_write_storage::<[Vec4<u32>]>(
    ZeroedArrayByArrayLength((cases * input.out_stride) as usize),
    gpu,
    "out",
  );
  let hasher = PipelineHasher::default().with_hash(name);
  let pipeline = gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(64);
      let u = builder.bind_by(&uniform).load().expand();
      let cps = builder.bind_by(&cps);
      let params = builder.bind_by(&params);
      let out = builder.bind_by(&out);
      let case = builder.global_invocation_id().x();
      if_by(case.greater_equal_than(val(cases)), do_return);
      kernel(&KernelCtx {
        u,
        cps,
        params,
        out,
        case,
        param_stride: input.param_stride,
        out_stride: input.out_stride,
      });
      builder
    });
  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
      .with_bind(&uniform)
      .with_bind(&cps)
      .with_bind(&params)
      .with_bind(&out)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(cases.div_ceil(64), 1, 1);
  });
  let result = encoder.read_storage_array(&gpu.device, &out);
  gpu.submit_encoder(encoder);
  result.await.unwrap()
}

fn as_f32(v: Vec4<u32>) -> Vec4<f64> {
  let f = f32::from_bits;
  Vec4::new(f(v.x), f(v.y), f(v.z), f(v.w)).into_f64()
}

fn round_f32(p: Vec4<f64>) -> Vec4<f64> {
  p.into_f32().into_f64()
}

fn rounded_surface(s: &RationalBezierSurface) -> RationalBezierSurface {
  let cp = s.control_points.iter().map(|p| round_f32(*p)).collect();
  RationalBezierSurface::new(s.degree_u, s.degree_v, cp)
}

fn test_uniform() -> TessUniform {
  TessUniform::new(&test_camera(), &TessellationConfig::default())
}

fn random_dyadic(rng: &mut TestRng, max_level: u32) -> u32 {
  let level = (rng.unit() * (max_level + 1) as f64) as u32;
  let index = (rng.unit() * ((1 << level) + 1) as f64) as u32;
  (index << (PARAM_BITS - level)).min(PARAM_ONE)
}

fn random_segment(rng: &mut TestRng, min_level: u32, max_level: u32) -> Segment {
  let level = min_level + (rng.unit() * (max_level - min_level + 1) as f64) as u32;
  let index = (rng.unit() * (1u64 << level) as f64) as u32;
  Segment { level, index }
}

fn close(a: Vec4<f64>, b: Vec4<f64>, scale: f64, tolerance: f64) -> bool {
  (a - b).length() <= scale * tolerance
}

fn max_norm(points: &[Vec4<f64>]) -> f64 {
  points.iter().map(|p| p.length()).fold(0., f64::max)
}

/// The ancestor of the CPU segment at `level` as a pair for comparison.
fn pair(s: Segment) -> (u32, u32) {
  (s.level, s.index)
}

#[pollster::test]
async fn dyadic_ops_are_exact() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut rng = TestRng(1);
  let mut params = Vec::new();
  let mut expected = Vec::new();
  for case in 0..2000 {
    let s = random_segment(&mut rng, 0, PARAM_BITS);
    let ancestor_level = (rng.unit() * (s.level + 1) as f64) as u32;
    let p = if case % 3 == 0 {
      random_dyadic(&mut rng, PARAM_BITS)
    } else {
      (rng.unit() * PARAM_ONE as f64) as u32
    };
    let level = (rng.unit() * (PARAM_BITS + 1) as f64) as u32;
    params.push(Vec4::new(s.level, s.index, ancestor_level, p));
    params.push(Vec4::new(level, 0, 0, 0));
    let containing = Segment::containing(level, p);
    expected.push([
      Vec4::new(
        s.start(),
        s.end(),
        s.reversed().index,
        s.ancestor(ancestor_level).index,
      ),
      Vec4::new(
        containing.is_some() as u32,
        containing.map_or(p >> (PARAM_BITS - level), |c| c.index),
        param_level(p),
        param_level(s.start()),
      ),
    ]);
  }
  let input = KernelInput {
    cps: vec![Vec4::zero()],
    params,
    param_stride: 2,
    out_stride: 2,
  };
  let out = run_kernel(&gpu, "dyadic", &input, test_uniform(), |cx| {
    let a = cx.param(0);
    let b = cx.param(1);
    let s = ShaderSegment::new(a.x(), a.y());
    let (inside, c) = ShaderSegment::containing(b.x(), a.w());
    let first = uvec4(
      s.start(),
      s.end(),
      s.reversed().index,
      s.ancestor(a.z()).index,
    );
    let second = uvec4(
      inside.select(val(1u32), val(0)),
      c.index,
      shader_param_level(a.w()),
      shader_param_level(s.start()),
    );
    cx.write_u(0, first);
    cx.write_u(1, second);
  })
  .await;
  for (case, e) in expected.iter().enumerate() {
    assert_eq!(out[case * 2], e[0], "case {case}");
    assert_eq!(out[case * 2 + 1], e[1], "case {case}");
  }
}

/// A random rational curve of `degree` (up to `MAX_GPU_DEGREE`) around `center`.
fn random_curve(rng: &mut TestRng, degree: usize, rational: bool) -> RationalBezierCurve3 {
  let center = Vec3::new(rng.range(-3., 3.), rng.range(-3., 3.), rng.range(-30., -3.));
  let points = (0..=degree)
    .map(|i| {
      let t = i as f64 / degree as f64;
      let p = center
        + Vec3::new(4. * t - 2., rng.range(-1., 1.), rng.range(-1., 1.)) * rng.range(0.1, 1.);
      let w = if rational { rng.range(0.5, 1.5) } else { 1. };
      round_f32(Vec4::from_euclid(p, w))
    })
    .collect();
  RationalBezierCurve::new(points)
}

#[pollster::test]
async fn curves_match_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut rng = TestRng(2);
  let mut cps = Vec::new();
  let mut params = Vec::new();
  let mut cases = Vec::new();
  for case in 0..400 {
    let degree = [1, 2, 3, 5, 9, MAX_GPU_DEGREE][case % 6];
    let curve = random_curve(&mut rng, degree, case % 2 == 0);
    let segment = random_segment(&mut rng, 0, 12);
    let t = random_dyadic(&mut rng, 12);
    params.push(Vec4::new(
      cps.len() as u32,
      degree as u32,
      segment.level,
      segment.index,
    ));
    params.push(Vec4::new(t, 0, 0, 0));
    cps.extend(curve.control_points.iter().map(|p| p.into_f32()));
    cases.push((curve, segment, t));
  }
  // out: sub curve points, bounds (min, max, errors), eval, eval with derivative
  let stride = MAX_CURVE_POINTS as u32 + 6;
  let input = KernelInput {
    cps,
    params,
    param_stride: 2,
    out_stride: stride,
  };
  let out = run_kernel(&gpu, "curves", &input, test_uniform(), |cx| {
    let a = cx.param(0);
    let t = param_to_f32(cx.param(1).x());
    let (offset, degree) = (a.x(), a.y());
    let segment = ShaderSegment::new(a.z(), a.w());
    let buf = make_curve_buffer();
    let load = || {
      for_range(val(0), degree + val(1), |i, _| {
        buf.index(i).store(cx.cps.index(offset + i).load());
      });
    };
    load();
    sub_curve_in_place(
      &buf,
      degree,
      param_to_f32(segment.start()),
      param_to_f32(segment.end()),
    );
    for_range(val(0), degree + val(1), |i, _| {
      cx.write_f(i, buf.index(i).load());
    });
    let bounds = shader_curve_bounds(&buf, degree);
    let base = MAX_CURVE_POINTS as u32;
    cx.write_f(base, vec3_to_4(bounds.min, val(0.)));
    cx.write_f(base + 1, vec3_to_4(bounds.max, val(0.)));
    let errors = fvec4(
      bounds.chord_error,
      bounds.polyline_coefficient,
      bounds.polyline_error(val(1)),
      bounds.polyline_error(val(4)),
    );
    cx.write_f(base + 2, errors);
    load();
    cx.write_f(base + 3, de_casteljau_in_place(&buf, degree, t));
    load();
    let (value, derivative) = de_casteljau_with_derivative_in_place(&buf, degree, t);
    cx.write_f(base + 4, value);
    cx.write_f(base + 5, derivative);
  })
  .await;

  for (case, (curve, segment, t)) in cases.iter().enumerate() {
    let o = |k: usize| as_f32(out[case * stride as usize + k]);
    let sub = curve.sub_curve(param_to_f64(segment.start()), param_to_f64(segment.end()));
    let scale = max_norm(&curve.control_points);
    for (i, p) in sub.control_points.iter().enumerate() {
      assert!(
        close(o(i), *p, scale, 1e-5),
        "case {case} cp {i}: {:?} {p:?}",
        o(i)
      );
    }
    let b = curve_bounds(&sub);
    let base = MAX_CURVE_POINTS;
    let tolerance = scale * 2e-5;
    assert!((o(base).xyz() - b.min).length() <= tolerance, "case {case}");
    assert!(
      (o(base + 1).xyz() - b.max).length() <= tolerance,
      "case {case}"
    );
    let errors = o(base + 2);
    let reference = [
      b.chord_error,
      b.polyline_coefficient,
      b.polyline_error(1),
      b.polyline_error(4),
    ];
    for (gpu_value, cpu_value) in [errors.x, errors.y, errors.z, errors.w]
      .iter()
      .zip(reference)
    {
      assert!(
        (gpu_value - cpu_value).abs() <= cpu_value * 1e-3 + tolerance,
        "case {case}: {errors:?} {reference:?}"
      );
    }
    let t = param_to_f64(*t);
    let value = curve.eval_homogeneous(t);
    assert!(close(o(base + 3), value, scale, 1e-5), "case {case}");
    let (v, d) = de_casteljau_with_derivative(&curve.control_points, t);
    assert!(close(o(base + 4), v, scale, 1e-5), "case {case}");
    assert!(
      close(o(base + 5), d, scale * curve.degree() as f64, 1e-4),
      "case {case}"
    );
  }
}

/// Random camera space surfaces in f32, with transposed and reversed variants.
fn random_items(rng: &mut TestRng, count: usize) -> Vec<TessellationItem> {
  (0..count)
    .map(|case| {
      let s = random_surface(rng, case % 2 == 0);
      let s = match case % 4 {
        1 => transpose(&s),
        2 => s.sub_patch((1., 0.), (0., 1.)),
        _ => s,
      };
      let to_camera = placement(rng.range(2.5, 40.), rng.range(-1.3, 1.3));
      TessellationItem {
        instance: 0,
        patch: case as u32,
        surface: rounded_surface(&s.transform(to_camera)),
      }
    })
    .collect()
}

fn upload_items(items: &[TessellationItem]) -> (Vec<Vec4<f32>>, Vec<u32>) {
  let mut cps = Vec::new();
  let offsets = items
    .iter()
    .map(|item| {
      let offset = cps.len() as u32;
      cps.extend(item.surface.control_points.iter().map(|p| p.into_f32()));
      offset
    })
    .collect();
  (cps, offsets)
}

fn reader(
  cx: &KernelCtx,
  offset: Node<u32>,
  degree_u: Node<u32>,
) -> impl Fn(Node<u32>, Node<u32>) -> Node<Vec4<f32>> + '_ {
  move |i, j| cx.cps.index(offset + j * (degree_u + val(1)) + i).load()
}

#[pollster::test]
async fn sub_patches_and_surfaces_match_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut rng = TestRng(3);
  let items = random_items(&mut rng, 160);
  let (cps, offsets) = upload_items(&items);
  let mut params = Vec::new();
  let mut cases = Vec::new();
  for (item, offset) in items.iter().zip(offsets) {
    let s = &item.surface;
    let level = (rng.unit() * 5.) as u32;
    let size = 1 << level;
    let key = SubPatchKey {
      item: 0,
      level,
      ix: (rng.unit() * size as f64) as u32,
      iy: (rng.unit() * size as f64) as u32,
    };
    let (u, v) = (random_dyadic(&mut rng, 10), random_dyadic(&mut rng, 10));
    params.push(Vec4::new(offset, s.degree_u as u32, s.degree_v as u32, 0));
    params.push(Vec4::new(level, key.ix, key.iy, 0));
    params.push(Vec4::new(u, v, 0, 0));
    cases.push((key, u, v));
  }
  let stride = (MAX_CURVE_POINTS * MAX_CURVE_POINTS) as u32 + 7;
  let input = KernelInput {
    cps,
    params,
    param_stride: 3,
    out_stride: stride,
  };
  let out = run_kernel(&gpu, "sub_patches", &input, test_uniform(), |cx| {
    let a = cx.param(0);
    let key = cx.param(1);
    let uv = cx.param(2);
    let (n, m) = (a.y(), a.z());
    let cp = reader(cx, a.x(), n);
    let us = ShaderSegment::new(key.x(), key.y());
    let vs = ShaderSegment::new(key.x(), key.z());
    let u = (param_to_f32(us.start()), param_to_f32(us.end()));
    let v = (param_to_f32(vs.start()), param_to_f32(vs.end()));
    for_range(val(0), m + val(1), |j, _| {
      for_range(val(0), n + val(1), |i, _| {
        let p = sub_patch_point(&cp, n, m, i, j, u, v);
        cx.write_f(j * (n + val(1)) + i, p);
      });
    });
    let sub = |i: Node<u32>, j: Node<u32>| cx.read_f(j * (n + val(1)) + i);
    let bounds = shader_surface_bounds(&sub, n, m);
    let base = (MAX_CURVE_POINTS * MAX_CURVE_POINTS) as u32;
    cx.write_f(base, vec3_to_4(bounds.min, val(0.)));
    cx.write_f(base + 1, vec3_to_4(bounds.max, val(0.)));
    let errors = fvec4(
      bounds.quad_error,
      bounds.grid_coefficient,
      bounds.grid_error(val(1)),
      bounds.grid_error(val(4)),
    );
    cx.write_f(base + 2, errors);
    let (pu, pv) = (param_to_f32(uv.x()), param_to_f32(uv.y()));
    cx.write_f(base + 3, surface_eval(&cp, n, m, pu, pv));
    let (p, du, dv) = surface_eval_with_partials(&cp, n, m, pu, pv);
    cx.write_f(base + 4, vec3_to_4(p, val(1.)));
    cx.write_f(base + 5, vec3_to_4(du, val(0.)));
    cx.write_f(base + 6, vec3_to_4(dv, val(0.)));
  })
  .await;

  for (case, (item, (key, u, v))) in items.iter().zip(&cases).enumerate() {
    let o = |k: usize| as_f32(out[case * stride as usize + k]);
    let s = &item.surface;
    let (us, vs) = (key.u_segment(), key.v_segment());
    let sub = s.sub_patch(
      (param_to_f64(us.start()), param_to_f64(us.end())),
      (param_to_f64(vs.start()), param_to_f64(vs.end())),
    );
    let scale = max_norm(&s.control_points);
    for (i, p) in sub.control_points.iter().enumerate() {
      assert!(
        close(o(i), *p, scale, 2e-5),
        "case {case} cp {i}: {:?} {p:?}",
        o(i)
      );
    }
    let b = surface_bounds(&sub);
    let base = MAX_CURVE_POINTS * MAX_CURVE_POINTS;
    let tolerance = scale * 5e-5;
    assert!((o(base).xyz() - b.min).length() <= tolerance, "case {case}");
    assert!(
      (o(base + 1).xyz() - b.max).length() <= tolerance,
      "case {case}"
    );
    let errors = o(base + 2);
    let reference = [
      b.quad_error,
      b.grid_coefficient,
      b.grid_error(1),
      b.grid_error(4),
    ];
    for (gpu_value, cpu_value) in [errors.x, errors.y, errors.z, errors.w]
      .iter()
      .zip(reference)
    {
      assert!(
        (gpu_value - cpu_value).abs() <= cpu_value * 1e-3 + tolerance,
        "case {case}: {errors:?} {reference:?}"
      );
    }
    let (u, v) = (param_to_f64(*u), param_to_f64(*v));
    assert!(close(o(base + 3), s.eval_homogeneous(u, v), scale, 1e-5));
    let (p, du, dv) = s.eval_with_partials(u, v);
    assert!(
      (o(base + 4).xyz() - p).length() <= scale * 1e-5,
      "case {case}"
    );
    let derivative_scale = scale * 20.;
    assert!(
      (o(base + 5).xyz() - du).length() <= derivative_scale * 1e-4,
      "case {case}"
    );
    assert!(
      (o(base + 6).xyz() - dv).length() <= derivative_scale * 1e-4,
      "case {case}"
    );
  }
}

/// Lines of random items, a third of them patch boundaries.
fn random_line(rng: &mut TestRng) -> LineKey {
  let axis = if rng.unit() < 0.5 { Axis::U } else { Axis::V };
  let coord = match (rng.unit() * 3.) as u32 {
    0 => 0,
    1 => PARAM_ONE,
    _ => random_dyadic(rng, 8),
  };
  LineKey::new(axis, coord)
}

#[pollster::test]
async fn line_curves_match_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut rng = TestRng(4);
  let items = random_items(&mut rng, 300);
  let (cps, offsets) = upload_items(&items);
  let mut params = Vec::new();
  let mut lines = Vec::new();
  for (index, (item, offset)) in items.iter().zip(offsets).enumerate() {
    let line = random_line(&mut rng);
    let s = &item.surface;
    params.push(Vec4::new(offset, s.degree_u as u32, s.degree_v as u32, 0));
    params.push(Vec4::new((line.axis == Axis::V) as u32, line.coord, 0, 0));
    lines.push((index as u32, line));
  }
  let stride = MAX_CURVE_POINTS as u32 + 1;
  let input = KernelInput {
    cps,
    params,
    param_stride: 2,
    out_stride: stride,
  };
  let out = run_kernel(&gpu, "lines", &input, test_uniform(), |cx| {
    let a = cx.param(0);
    let l = cx.param(1);
    let (n, m) = (a.y(), a.z());
    let cp = reader(cx, a.x(), n);
    let points = make_curve_buffer();
    let tmp = make_curve_buffer();
    let curve = load_line_curve(&cp, n, m, l.x().equals(val(1)), l.y(), &points, &tmp);
    for_range(val(0), curve.degree + val(1), |i, _| {
      cx.write_f(i, points.index(i).load());
    });
    let info = uvec4(
      curve.degree,
      curve.reversed.select(val(1u32), val(0)),
      curve.level,
      val(0u32),
    );
    cx.write_u(MAX_CURVE_POINTS as u32, info);
  })
  .await;

  let mut t = Tessellator::new(&items, test_camera(), TessellationConfig::default());
  let mut boundaries = 0;
  for (case, (item, line)) in lines.iter().enumerate() {
    let o = |k: usize| out[case * stride as usize + k];
    let curve = t.line_curve(*item, *line).clone();
    let info = o(MAX_CURVE_POINTS);
    assert_eq!(info.x as usize, curve.curve.degree(), "case {case}");
    assert_eq!(info.y == 1, curve.reversed, "case {case}");
    assert_eq!(info.z, line.level(), "case {case}");
    let scale = max_norm(&curve.curve.control_points);
    for (i, p) in curve.curve.control_points.iter().enumerate() {
      let gpu_point = as_f32(o(i));
      if line.is_patch_boundary() {
        assert_eq!(gpu_point, *p, "case {case} point {i}");
      } else {
        assert!(close(gpu_point, *p, scale, 1e-5), "case {case} point {i}");
      }
    }
    boundaries += line.is_patch_boundary() as usize;
  }
  assert!(boundaries > 50);
}

/// `linearization`, `flat_ancestor` and `locate` agree with the CPU, except for decisions that
/// flip when the CPU tolerance is changed by a relative 1e-3.
#[pollster::test]
async fn linearization_queries_match_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut rng = TestRng(5);
  let items = random_items(&mut rng, 400);
  let (cps, offsets) = upload_items(&items);
  let config = TessellationConfig::default();
  let mut params = Vec::new();
  let mut queries = Vec::new();
  for (index, (item, offset)) in items.iter().zip(offsets).enumerate() {
    let s = &item.surface;
    for _ in 0..4 {
      let line = random_line(&mut rng);
      let segment = random_segment(&mut rng, line.level(), config.max_level);
      let p = random_dyadic(&mut rng, 14);
      params.push(Vec4::new(offset, s.degree_u as u32, s.degree_v as u32, 0));
      params.push(Vec4::new(
        (line.axis == Axis::V) as u32,
        line.coord,
        segment.level,
        segment.index,
      ));
      params.push(Vec4::new(p, 0, 0, 0));
      queries.push((index as u32, line, segment, p));
    }
  }
  let input = KernelInput {
    cps,
    params,
    param_stride: 3,
    out_stride: 2,
  };
  let out = run_kernel(&gpu, "linearization", &input, test_uniform(), |cx| {
    let a = cx.param(0);
    let l = cx.param(1);
    let p = cx.param(2).x();
    let (n, m) = (a.y(), a.z());
    let cp = reader(cx, a.x(), n);
    let points = make_curve_buffer();
    let tmp = make_curve_buffer();
    let curve = load_line_curve(&cp, n, m, l.x().equals(val(1)), l.y(), &points, &tmp);
    let segment = curve.canonical_segment(ShaderSegment::new(l.z(), l.w()));
    let pieces = linearization(&cx.u, &points, curve.degree, segment, &tmp);
    let (flat, flat_pieces) = flat_ancestor(&cx.u, &points, curve, segment, &tmp);
    let (found, piece) = locate(&cx.u, &points, curve, p, &tmp);
    let first = uvec4(pieces, flat.level, flat.index, flat_pieces);
    let second = uvec4(
      found.select(val(1u32), val(0)),
      piece.level,
      piece.index,
      val(0u32),
    );
    cx.write_u(0, first);
    cx.write_u(1, second);
  })
  .await;

  let answers = |t: &mut Tessellator, (item, line, segment, p): (u32, LineKey, Segment, u32)| {
    let segment = t.line_curve(item, line).canonical_segment(segment);
    let pieces = t.linearization_of(item, line, segment).unwrap_or(0);
    let flat = t
      .flat_ancestor_of(item, line, segment)
      .map_or((0, 0), |(s, m)| (s.index, m));
    let located = t.locate_on(item, line, p).map(pair);
    (pieces, flat, located)
  };
  let mut t = Tessellator::new(&items, test_camera(), config);
  let perturbed = [0.999, 1.001].map(|f| TessellationConfig {
    pixel_error: config.pixel_error * f,
    ..config
  });
  let mut near_threshold = 0;
  let mut found = 0;
  for (case, &query) in queries.iter().enumerate() {
    let (first, second) = (out[case * 2], out[case * 2 + 1]);
    let gpu_located = (second.x == 1).then_some((second.y, second.z));
    let gpu_answer = (first.x, (first.z, first.w), gpu_located);
    found += gpu_located.is_some() as usize;
    let cpu = answers(&mut t, query);
    // A missing flat ancestor reports level 0 on the GPU.
    let gpu_answer = if first.w == 0 {
      (gpu_answer.0, (0, 0), gpu_answer.2)
    } else {
      gpu_answer
    };
    if gpu_answer == cpu {
      continue;
    }
    let explained = perturbed.iter().any(|config| {
      let mut t = Tessellator::new(&items, test_camera(), *config);
      answers(&mut t, query) == gpu_answer
    });
    assert!(
      explained,
      "case {case} {query:?}: gpu {gpu_answer:?} cpu {cpu:?}"
    );
    near_threshold += 1;
  }
  println!(
    "located {found} of {} queries, {near_threshold} near threshold",
    queries.len()
  );
  assert!(found > queries.len() / 10, "{found}");
  assert!(near_threshold < queries.len() / 100, "{near_threshold}");
}
