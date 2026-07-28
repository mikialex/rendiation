use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

use crate::bezier_curve3d_device::*;
use crate::curve3d::*;

fn test_curve_deg1() -> RationalBezierCurve3d<f32> {
  RationalBezierCurve3d::from_unweighted(
    vec![Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 0.3)],
    1,
  )
}

fn test_curve_deg2() -> RationalBezierCurve3d<f32> {
  RationalBezierCurve3d::from_unweighted(
    vec![
      Vec3::new(-1.0, 0.0, 0.0),
      Vec3::new(0.0, 1.0, 0.8),
      Vec3::new(1.0, 0.0, 0.0),
    ],
    2,
  )
}

fn test_curve_deg3() -> RationalBezierCurve3d<f32> {
  RationalBezierCurve3d::from_unweighted(
    vec![
      Vec3::new(-1.0, 0.0, 0.0),
      Vec3::new(-0.5, 1.0, 0.3),
      Vec3::new(0.5, 1.0, -0.3),
      Vec3::new(1.0, 0.0, 0.0),
    ],
    3,
  )
}

fn test_curve_deg5() -> RationalBezierCurve3d<f32> {
  let points: Vec<Vec3<f32>> = (0..6)
    .map(|i| {
      let x = i as f32 * 0.4 - 1.0;
      let z = (x * 1.5).sin() * 0.8;
      Vec3::new(x, (i % 2) as f32 * 0.6, z)
    })
    .collect();
  RationalBezierCurve3d::from_unweighted(points, 5)
}

fn test_curve_deg14() -> RationalBezierCurve3d<f32> {
  let points: Vec<Vec3<f32>> = (0..15)
    .map(|i| {
      let x = i as f32 * 0.14 - 1.0;
      let y = ((i as f32 - 7.0) * 0.3).sin() * 0.8;
      let z = (-(x * x) * 1.5).exp() * 0.5;
      Vec3::new(x, y, z)
    })
    .collect();
  RationalBezierCurve3d::from_unweighted(points, 14)
}

async fn run_gpu_test(curve: &RationalBezierCurve3d<f32>, sample_count: u32, eps: f32) {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let total_samples = sample_count as usize;

  let info = create_gpu_readonly_storage(&curve.to_gpu_info(), &gpu, "info");
  let cp = create_gpu_readonly_storage(&curve.to_gpu_control_points(), &gpu, "cp");
  let output = create_gpu_read_write_storage::<[Vec4<f32>]>(
    ZeroedArrayByArrayLength(total_samples),
    &gpu,
    "output",
  );

  let workgroup_size: u32 = 64;

  let pipeline =
    build_bezier_curve_bernstein_pipeline(&gpu, &info, &cp, &output, sample_count, workgroup_size);

  let dispatch_x = (total_samples as u32 + workgroup_size - 1) / workgroup_size;
  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
      .with_bind(&info)
      .with_bind(&cp)
      .with_bind(&output)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(dispatch_x, 1, 1);
  });

  let result = encoder.read_buffer(&gpu.device, &output);
  gpu.submit_encoder(encoder);
  let result = result.await.unwrap();
  let gpu_positions: Vec<Vec4<f32>> =
    <[Vec4<f32>]>::from_bytes_into_boxed(&result.read_raw()).into_vec();

  for (idx, gpu_pt) in gpu_positions.iter().enumerate().take(total_samples) {
    let t = if sample_count > 1 {
      idx as f32 / (sample_count - 1) as f32
    } else {
      0.5
    };

    let cpu_pt = curve.evaluate(t);
    let gpu_p = Vec3::new(gpu_pt.x, gpu_pt.y, gpu_pt.z);

    assert!(
      (cpu_pt.x - gpu_p.x).abs() < eps
        && (cpu_pt.y - gpu_p.y).abs() < eps
        && (cpu_pt.z - gpu_p.z).abs() < eps,
      "deg({}) mismatch at t={t:.3} idx={idx}: CPU {cpu_pt:?} vs GPU {gpu_p:?}",
      curve.degree(),
    );
  }
}

#[pollster::test]
async fn curve_gpu_vs_cpu_deg1() {
  run_gpu_test(&test_curve_deg1(), 8, 1e-4).await;
}

#[pollster::test]
async fn curve_gpu_vs_cpu_deg2() {
  run_gpu_test(&test_curve_deg2(), 8, 1e-4).await;
}

#[pollster::test]
async fn curve_gpu_vs_cpu_deg3() {
  run_gpu_test(&test_curve_deg3(), 16, 1e-4).await;
}

#[pollster::test]
async fn curve_gpu_vs_cpu_deg5() {
  run_gpu_test(&test_curve_deg5(), 16, 1e-4).await;
}

#[pollster::test]
async fn curve_gpu_vs_cpu_deg14() {
  run_gpu_test(&test_curve_deg14(), 8, 1e-4).await;
}
