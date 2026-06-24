use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

use crate::bezier_curve3d_device::storage::*;
use crate::bezier_device_shared::*;

/// Evaluate a rational Bézier curve point via Bernstein basis.
///
/// Precomputes powers `t^i` iteratively, then performs the single-loop
/// accumulation using binomial coefficient lookup.
fn evaluate_bernstein_curve(
  t: Node<f32>,
  degree: Node<u32>,
  binomial: ShaderReadonlyPtrOf<[f32]>,
  cp_data: ShaderReadonlyPtrOf<[Vec4<f32>; MAX_GPU_CURVE_CONTROL_POINTS]>,
) -> Node<Vec4<f32>> {
  let s = val(1.0_f32) - t;

  // Precompute powers: t^i, (1-t)^i for i in 0..=MAX_GPU_DEGREE
  let t_pow: ShaderPtrOf<[f32; 15]> = make_local_var::<[f32; 15]>();
  let s_pow: ShaderPtrOf<[f32; 15]> = make_local_var::<[f32; 15]>();

  t_pow.index(val(0u32)).store(val(1.0_f32));
  s_pow.index(val(0u32)).store(val(1.0_f32));

  {
    let max_deg = val((MAX_GPU_DEGREE + 1) as u32);
    let pow_range = ForRange::ranged((val(1u32), max_deg).into());
    pow_range.for_each(|i, _| {
      let prev = i - val(1u32);
      t_pow.index(i).store(t_pow.index(prev).load() * t);
      s_pow.index(i).store(s_pow.index(prev).load() * s);
    });
  }

  // binomial[(degree - 1) * 16 + k]
  let get_binomial = |deg: Node<u32>, k: Node<u32>| -> Node<f32> {
    binomial.index((deg - val(1u32)) * val(16u32) + k).load()
  };

  // Single summation (no tensor product)
  let sum: ShaderPtrOf<Vec4<f32>> = zeroed_val::<Vec4<f32>>().make_local_var();
  let limit: Node<u32> = degree + val(1u32);

  {
    let range = ForRange::ranged((val(0u32), limit).into());
    range.for_each(|i, _| {
      let b = get_binomial(degree, i) * t_pow.index(i).load() * s_pow.index(degree - i).load();
      let c = cp_data.index(i).load();
      sum.store(sum.load() + c * b);
    });
  }

  sum.load()
}

/// Build a compute pipeline that evaluates a Bézier curve at `sample_count`
/// points using the Bernstein basis with binomial coefficient lookup.
///
/// Supports arbitrary degree up to `MAX_GPU_DEGREE` (14). Output is
/// `Vec4<f32>` per sample: `(position.xyz, w)`.
pub fn build_bezier_curve_bernstein_pipeline(
  gpu: &GPU,
  info: &StorageBufferReadonlyDataView<GpuBezierCurveInfo>,
  control_points: &StorageBufferReadonlyDataView<GpuBezierCurveControlPoints>,
  binomial: &StorageBufferReadonlyDataView<[f32]>,
  output: &StorageBufferDataView<[Vec4<f32>]>,
  sample_count: u32,
  workgroup_size: u32,
) -> GPUComputePipeline {
  let hasher = shader_hasher_from_marker_ty!(BezierCurveEval).with_hash(workgroup_size);

  gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(workgroup_size);

      let info = builder.bind_by(info);
      let cp = builder.bind_by(control_points);
      let binomial = builder.bind_by(binomial);
      let output = builder.bind_by(output);

      let gid = builder.global_invocation_id().x();
      let total = val(sample_count);

      if_by(gid.greater_equal_than(total), || {
        do_return();
      });

      let t = gid.into_f32() / val((sample_count.max(2) - 1) as f32);
      let degree = info.degree().load();

      // de Casteljau fast-path for degree 1–3
      if_by(degree.less_than(val(4u32)), || {
        let cp_data = cp.data();

        let sw: ShaderPtrOf<Vec4<f32>> = make_local_var::<Vec4<f32>>();
        switch_by(degree)
          .case(1, || {
            let p0 = cp_data.index(val(0u32)).load();
            let p1 = cp_data.index(val(1u32)).load();
            sw.store(de_casteljau_curve_deg1_fn(p0, p1, t));
          })
          .case(2, || {
            let p0 = cp_data.index(val(0u32)).load();
            let p1 = cp_data.index(val(1u32)).load();
            let p2 = cp_data.index(val(2u32)).load();
            sw.store(de_casteljau_curve_deg2_fn(p0, p1, p2, t));
          })
          .case(3, || {
            let p0 = cp_data.index(val(0u32)).load();
            let p1 = cp_data.index(val(1u32)).load();
            let p2 = cp_data.index(val(2u32)).load();
            let p3 = cp_data.index(val(3u32)).load();
            sw.store(de_casteljau_curve_deg3_fn(p0, p1, p2, p3, t));
          })
          .end_with_default(|| {});

        let sw_val = sw.load();
        let w: Node<f32> = sw_val.w();
        let p = Vec3::new(sw_val.x() / w, sw_val.y() / w, sw_val.z() / w);
        let p4: Node<Vec4<f32>> = (p.x(), p.y(), p.z(), w).into();
        output.index(gid).store(p4);
        do_return();
      });

      let sw = evaluate_bernstein_curve(t, degree, binomial, cp.data());

      // Project from homogeneous to Cartesian
      let w: Node<f32> = sw.w();
      let p = Vec3::new(sw.x() / w, sw.y() / w, sw.z() / w);
      let p4: Node<Vec4<f32>> = (p.x(), p.y(), p.z(), w).into();
      output.index(gid).store(p4);

      builder
    })
}
