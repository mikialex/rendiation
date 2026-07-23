use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

use crate::bezier_device_shared::*;
use crate::bezier_surface_device::storage::*;

/// Evaluate a rational Bézier surface point via Bernstein basis tensor product.
///
/// Precompute powers `t^i` iteratively (avoiding `pow()`), then performs the
/// double-loop accumulation using binomial coefficient lookup.
fn evaluate_bernstein_surface(
  u: Node<f32>,
  v: Node<f32>,
  u_degree: Node<u32>,
  v_degree: Node<u32>,
  cp_data: ShaderReadonlyPtrOf<[Vec4<f32>; MAX_GPU_CONTROL_POINTS]>,
) -> Node<Vec4<f32>> {
  let su = val(1.0_f32) - u;
  let sv = val(1.0_f32) - v;

  // Precompute powers: u^i, (1-u)^i, v^j, (1-v)^j for i,j in 0..=MAX_GPU_DEGREE
  let u_pow: ShaderPtrOf<[f32; 15]> = make_local_var::<[f32; 15]>();
  let su_pow: ShaderPtrOf<[f32; 15]> = make_local_var::<[f32; 15]>();
  let v_pow: ShaderPtrOf<[f32; 15]> = make_local_var::<[f32; 15]>();
  let sv_pow: ShaderPtrOf<[f32; 15]> = make_local_var::<[f32; 15]>();

  u_pow.index(val(0u32)).store(val(1.0_f32));
  su_pow.index(val(0u32)).store(val(1.0_f32));
  v_pow.index(val(0u32)).store(val(1.0_f32));
  sv_pow.index(val(0u32)).store(val(1.0_f32));

  {
    let max_deg = val((MAX_GPU_DEGREE + 1) as u32);
    let pow_range = ForRange::ranged((val(1u32), max_deg).into());
    pow_range.for_each(|i, _| {
      let prev = i - val(1u32);
      u_pow.index(i).store(u_pow.index(prev).load() * u);
      su_pow.index(i).store(su_pow.index(prev).load() * su);
      v_pow.index(i).store(v_pow.index(prev).load() * v);
      sv_pow.index(i).store(sv_pow.index(prev).load() * sv);
    });
  }

  let binomial = global_const_val(BINOMIAL_COEFFICIENTS);

  // binomial[(degree - 1) * 16 + k]
  let get_binomial = |deg: Node<u32>, k: Node<u32>| -> Node<f32> {
    binomial.index((deg - val(1u32)) * val(16u32) + k)
  };

  // Tensor-product accumulation
  let sum: ShaderPtrOf<Vec4<f32>> = zeroed_val::<Vec4<f32>>().make_local_var();
  let u_cp_stride: Node<u32> = u_degree + val(1u32);
  let v_limit: Node<u32> = v_degree + val(1u32);
  let u_limit: Node<u32> = u_degree + val(1u32);

  {
    let v_range = ForRange::ranged((val(0u32), v_limit).into());
    v_range.for_each(|v_j, _| {
      let bv =
        get_binomial(v_degree, v_j) * v_pow.index(v_j).load() * sv_pow.index(v_degree - v_j).load();
      let u_range = ForRange::ranged((val(0u32), u_limit).into());
      u_range.for_each(|u_i, _| {
        let bu = get_binomial(u_degree, u_i)
          * u_pow.index(u_i).load()
          * su_pow.index(u_degree - u_i).load();
        let cp_idx = v_j * u_cp_stride + u_i;
        let c = cp_data.index(cp_idx).load();
        sum.store(sum.load() + c * bu * bv);
      });
    });
  }

  sum.load()
}

/// Build (or retrieve from cache) a compute pipeline that evaluates a Bézier
/// surface at a grid of `(sample_count × sample_count)` points using the
/// Bernstein basis with binomial coefficient lookup.
///
/// Supports arbitrary degree up to `MAX_GPU_DEGREE` (14). Output is
/// `Vec4<f32>` per sample: `(position.xyz, w)`.
pub fn build_bezier_bernstein_pipeline(
  gpu: &GPU,
  info: &StorageBufferReadonlyDataView<GpuBezierSurfaceInfo>,
  control_points: &StorageBufferReadonlyDataView<GpuBezierControlPoints>,
  output: &StorageBufferDataView<[Vec4<f32>]>,
  sample_count: u32,
  workgroup_size: u32,
) -> GPUComputePipeline {
  let mut hasher = PipelineHasher::default();
  hasher.hash(workgroup_size);

  gpu
    .device
    .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder = builder.with_config_work_group_size(workgroup_size);

      let info = builder.bind_by(info);
      let cp = builder.bind_by(control_points);
      let output = builder.bind_by(output);

      let gid = builder.global_invocation_id().x();
      let total = val(sample_count * sample_count);

      if_by(gid.greater_equal_than(total), || {
        do_return();
      });

      let u_idx = gid % val(sample_count);
      let v_idx = gid / val(sample_count);
      let u = u_idx.into_f32() / val((sample_count.max(2) - 1) as f32);
      let v = v_idx.into_f32() / val((sample_count.max(2) - 1) as f32);

      let u_degree = info.u_degree().load();
      let v_degree = info.v_degree().load();

      // --- de Casteljau fast-path for degree 1–3 ---
      if_by(
        u_degree
          .less_than(val(4u32))
          .and(v_degree.less_than(val(4u32))),
        || {
          let cp_data = cp.data();
          let u_n = u_degree + val(1u32);
          let v_n = v_degree + val(1u32);

          // Evaluate each v-row at u using de Casteljau
          let intermediates: ShaderPtrOf<[Vec4<f32>; 4]> = make_local_var::<[Vec4<f32>; 4]>();
          let row_result: ShaderPtrOf<Vec4<f32>> = make_local_var::<Vec4<f32>>();

          let v_range = ForRange::ranged((val(0u32), v_n).into());
          v_range.for_each(|v_j, _| {
            let base = v_j * u_n;

            switch_by(u_degree)
              .case(1, || {
                let p0 = cp_data.index(base).load();
                let p1 = cp_data.index(base + val(1u32)).load();
                row_result.store(de_casteljau_curve_deg1_fn(p0, p1, u));
              })
              .case(2, || {
                let p0 = cp_data.index(base).load();
                let p1 = cp_data.index(base + val(1u32)).load();
                let p2 = cp_data.index(base + val(2u32)).load();
                row_result.store(de_casteljau_curve_deg2_fn(p0, p1, p2, u));
              })
              .case(3, || {
                let p0 = cp_data.index(base).load();
                let p1 = cp_data.index(base + val(1u32)).load();
                let p2 = cp_data.index(base + val(2u32)).load();
                let p3 = cp_data.index(base + val(3u32)).load();
                row_result.store(de_casteljau_curve_deg3_fn(p0, p1, p2, p3, u));
              })
              .end_with_default(|| {});

            intermediates.index(v_j).store(row_result.load());
          });

          // Evaluate the intermediate column at v
          let sw: ShaderPtrOf<Vec4<f32>> = make_local_var::<Vec4<f32>>();
          switch_by(v_degree)
            .case(1, || {
              let q0 = intermediates.index(val(0u32)).load();
              let q1 = intermediates.index(val(1u32)).load();
              sw.store(de_casteljau_curve_deg1_fn(q0, q1, v));
            })
            .case(2, || {
              let q0 = intermediates.index(val(0u32)).load();
              let q1 = intermediates.index(val(1u32)).load();
              let q2 = intermediates.index(val(2u32)).load();
              sw.store(de_casteljau_curve_deg2_fn(q0, q1, q2, v));
            })
            .case(3, || {
              let q0 = intermediates.index(val(0u32)).load();
              let q1 = intermediates.index(val(1u32)).load();
              let q2 = intermediates.index(val(2u32)).load();
              let q3 = intermediates.index(val(3u32)).load();
              sw.store(de_casteljau_curve_deg3_fn(q0, q1, q2, q3, v));
            })
            .end_with_default(|| {});

          // Project from homogeneous to Cartesian
          let sw_val = sw.load();
          let w: Node<f32> = sw_val.w();
          let p = Vec3::new(sw_val.x() / w, sw_val.y() / w, sw_val.z() / w);
          let p4: Node<Vec4<f32>> = (p.x(), p.y(), p.z(), w).into();
          output.index(gid).store(p4);
          do_return();
        },
      );

      let sw = evaluate_bernstein_surface(u, v, u_degree, v_degree, cp.data());

      // Project from homogeneous to Cartesian
      let w: Node<f32> = sw.w();
      let p = Vec3::new(sw.x() / w, sw.y() / w, sw.z() / w);
      let p4: Node<Vec4<f32>> = (p.x(), p.y(), p.z(), w).into();
      output.index(gid).store(p4);

      builder
    })
}
