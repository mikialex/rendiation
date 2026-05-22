use crate::*;

fn test_surface_deg1x1() -> RationalBezierSurface<f32> {
  RationalBezierSurface::from_unweighted(
    vec![
      Vec3::new(-1.0, -1.0, 0.0),
      Vec3::new(1.0, -1.0, 0.0),
      Vec3::new(-1.0, 1.0, 0.5),
      Vec3::new(1.0, 1.0, 0.5),
    ],
    1,
    1,
  )
}

fn test_surface_deg2x2() -> RationalBezierSurface<f32> {
  RationalBezierSurface::from_unweighted(
    vec![
      Vec3::new(-1.0, -1.0, 0.0),
      Vec3::new(0.0, -1.0, 0.8),
      Vec3::new(1.0, -1.0, 0.0),
      Vec3::new(-1.0, 0.0, 0.8),
      Vec3::new(0.0, 0.0, 1.5),
      Vec3::new(1.0, 0.0, 0.8),
      Vec3::new(-1.0, 1.0, 0.0),
      Vec3::new(0.0, 1.0, 0.8),
      Vec3::new(1.0, 1.0, 0.0),
    ],
    2,
    2,
  )
}

fn test_surface_deg1x2() -> RationalBezierSurface<f32> {
  RationalBezierSurface::from_unweighted(
    vec![
      Vec3::new(-1.0, -1.0, 0.0),
      Vec3::new(1.0, -1.0, 0.3),
      Vec3::new(-1.0, 0.0, 0.5),
      Vec3::new(1.0, 0.0, 0.8),
      Vec3::new(-1.0, 1.0, 0.0),
      Vec3::new(1.0, 1.0, 0.3),
    ],
    1,
    2,
  )
}

fn test_surface_deg2x3() -> RationalBezierSurface<f32> {
  RationalBezierSurface::from_unweighted(
    (0..4)
      .flat_map(|v| {
        (0..3).map(move |u| {
          let x = u as f32 * 2.0 - 2.0;
          let y = v as f32 * 1.33 - 2.0;
          let z = (x * 0.8).cos() * (y * 0.6).sin() * 0.7;
          Vec3::new(x, y, z)
        })
      })
      .collect(),
    2,
    3,
  )
}

fn test_surface_deg3x2() -> RationalBezierSurface<f32> {
  RationalBezierSurface::from_unweighted(
    (0..3)
      .flat_map(|v| {
        (0..4).map(move |u| {
          let x = u as f32 * 1.33 - 2.0;
          let y = v as f32 * 2.0 - 2.0;
          let z = (x * 0.6).sin() * (y * 0.8).cos() * 0.7;
          Vec3::new(x, y, z)
        })
      })
      .collect(),
    3,
    2,
  )
}

fn test_surface_deg3() -> RationalBezierSurface<f32> {
  let points: Vec<Vec3<f32>> = vec![
    Vec3::new(-1.0, -1.0, 0.0),
    Vec3::new(-0.3, -1.0, 0.5),
    Vec3::new(0.3, -1.0, 0.5),
    Vec3::new(1.0, -1.0, 0.0),
    Vec3::new(-1.0, -0.3, 0.5),
    Vec3::new(-0.3, -0.3, 1.0),
    Vec3::new(0.3, -0.3, 1.0),
    Vec3::new(1.0, -0.3, 0.5),
    Vec3::new(-1.0, 0.3, 0.5),
    Vec3::new(-0.3, 0.3, 1.0),
    Vec3::new(0.3, 0.3, 1.0),
    Vec3::new(1.0, 0.3, 0.5),
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(-0.3, 1.0, 0.5),
    Vec3::new(0.3, 1.0, 0.5),
    Vec3::new(1.0, 1.0, 0.0),
  ];
  RationalBezierSurface::from_unweighted(points, 3, 3)
}

fn test_surface_deg5() -> RationalBezierSurface<f32> {
  let points: Vec<Vec3<f32>> = (0..6)
    .flat_map(|v| {
      (0..6).map(move |u| {
        let x = u as f32 * 0.4 - 1.0;
        let y = v as f32 * 0.4 - 1.0;
        let z = (x * 1.5).sin() * (y * 1.5).cos() * 0.8;
        Vec3::new(x, y, z)
      })
    })
    .collect();
  RationalBezierSurface::from_unweighted(points, 5, 5)
}

fn test_surface_deg4x4() -> RationalBezierSurface<f32> {
  let points: Vec<Vec3<f32>> = (0..5)
    .flat_map(|v| {
      (0..5).map(move |u| {
        let x = u as f32 * 0.5 - 1.0;
        let y = v as f32 * 0.5 - 1.0;
        let z = (x * 1.2).cos() * (y * 1.2).sin() * 0.6;
        Vec3::new(x, y, z)
      })
    })
    .collect();
  RationalBezierSurface::from_unweighted(points, 4, 4)
}

fn test_surface_deg14x14() -> RationalBezierSurface<f32> {
  let points: Vec<Vec3<f32>> = (0..15)
    .flat_map(|v| {
      (0..15).map(move |u| {
        let x = u as f32 * 0.14 - 1.0;
        let y = v as f32 * 0.14 - 1.0;
        let z = (-(x * x + y * y) * 1.5).exp() * 0.8;
        Vec3::new(x, y, z)
      })
    })
    .collect();
  RationalBezierSurface::from_unweighted(points, 14, 14)
}

async fn run_gpu_test(surface: &RationalBezierSurface<f32>, sample_count: u32, eps: f32) {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let total_samples = (sample_count * sample_count) as usize;

  let info = create_gpu_readonly_storage(&surface.to_gpu_info(), &gpu);
  let cp = create_gpu_readonly_storage(&surface.to_gpu_control_points(), &gpu);
  let binomial = create_gpu_readonly_storage(BINOMIAL_COEFFICIENTS.as_slice(), &gpu);
  let output =
    create_gpu_read_write_storage::<[Vec4<f32>]>(ZeroedArrayByArrayLength(total_samples), &gpu);

  let workgroup_size: u32 = 64;

  let pipeline = build_bezier_bernstein_pipeline(
    &gpu,
    &info,
    &cp,
    &binomial,
    &output,
    sample_count,
    workgroup_size,
  );

  let dispatch_x = (total_samples as u32 + workgroup_size - 1) / workgroup_size;
  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
      .with_bind(&info)
      .with_bind(&cp)
      .with_bind(&binomial)
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
    let u_idx = idx as u32 % sample_count;
    let v_idx = idx as u32 / sample_count;
    let u = if sample_count > 1 {
      u_idx as f32 / (sample_count - 1) as f32
    } else {
      0.5
    };
    let v = if sample_count > 1 {
      v_idx as f32 / (sample_count - 1) as f32
    } else {
      0.5
    };

    let cpu_pt = surface.evaluate(u, v);
    let gpu_p = Vec3::new(gpu_pt.x, gpu_pt.y, gpu_pt.z);

    assert!(
      (cpu_pt.x - gpu_p.x).abs() < eps
        && (cpu_pt.y - gpu_p.y).abs() < eps
        && (cpu_pt.z - gpu_p.z).abs() < eps,
      "deg({},{}) mismatch at (u={u:.3}, v={v:.3}) idx={idx}: CPU {cpu_pt:?} vs GPU {gpu_p:?}",
      surface.u_degree(),
      surface.v_degree(),
    );
  }
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg3() {
  run_gpu_test(&test_surface_deg3(), 8, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg5() {
  run_gpu_test(&test_surface_deg5(), 12, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg1x1() {
  run_gpu_test(&test_surface_deg1x1(), 6, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg2x2() {
  run_gpu_test(&test_surface_deg2x2(), 8, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg1x2() {
  run_gpu_test(&test_surface_deg1x2(), 6, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg2x3() {
  run_gpu_test(&test_surface_deg2x3(), 8, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg3x2() {
  run_gpu_test(&test_surface_deg3x2(), 8, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg4x4() {
  run_gpu_test(&test_surface_deg4x4(), 6, 1e-4).await;
}

#[pollster::test]
async fn bernstein_gpu_vs_cpu_deg14x14() {
  run_gpu_test(&test_surface_deg14x14(), 3, 1e-4).await;
}
