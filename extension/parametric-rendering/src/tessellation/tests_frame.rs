//! Whole frame tests of the GPU tessellation against the CPU reference (plan M1, M3, M4, M6).

use std::time::Instant;

use parametric_normalized_model::*;
use parametric_tessellation::test_scenes::*;
use parametric_tessellation::verify::*;
use parametric_tessellation::*;

use super::*;
use crate::*;

const STEAM_MODEL: &str = "/Users/mikialex/dev/rendiation/temp/step-test/steam-controller.pnm";

/// Encodes and submits one frame, then reads it back.
async fn run_frame(gpu: &GPU, t: &mut GpuTessellator, camera: &Camera) -> GpuFrameReadback {
  let mut encoder = gpu.create_encoder();
  t.encode_frame(gpu, &mut encoder, camera);
  gpu.submit_encoder(encoder);
  t.read_back(gpu).await
}

/// Runs frames until the asynchronously read statistics report no overflow (the buffers grow
/// in between), returns the first frame without overflow.
async fn run_converged_frame(
  gpu: &GPU,
  t: &mut GpuTessellator,
  camera: &Camera,
) -> GpuFrameReadback {
  for _ in 0..40 {
    let frame = run_frame(gpu, t, camera).await;
    gpu.poll(true);
    if frame.stats.overflow == 0 {
      return frame;
    }
  }
  panic!("the buffers did not grow to fit the frame");
}

fn cpu_items(
  model: &ParametricNormalizedModel,
  camera: &Camera,
  untrimmed_only: bool,
) -> Vec<TessellationItem> {
  camera_space_items(model, camera, |p| !untrimmed_only || p.is_untrimmed()).0
}

/// Everything compared between the GPU frame and the CPU reference.
#[derive(Debug, Default)]
struct FrameComparison {
  cpu_leaves: usize,
  gpu_leaves: usize,
  leaf_set: LeafSetReport,
  cpu_work_items: Vec<usize>,
  gpu_work_items: Vec<u32>,
  cpu_leaves_per_k: Vec<usize>,
  gpu_leaves_per_k: Vec<u32>,
  watertight: WatertightReport,
  cpu_watertight: WatertightReport,
  error: ErrorReport,
  /// Max distance of a GPU vertex to the CPU reference position of the same leaf vertex, in
  /// pixels.
  max_position_offset: f64,
  max_normal_deviation: f64,
}

fn compare_frame(
  model: &ParametricNormalizedModel,
  camera: &Camera,
  config: TessellationConfig,
  untrimmed_only: bool,
  frame: &GpuFrameReadback,
  error_resolution: u32,
) -> FrameComparison {
  let items = cpu_items(model, camera, untrimmed_only);
  let mut t = Tessellator::new(&items, *camera, config);
  let leaves = t.run();
  let gpu_leaves = frame.cpu_leaves();
  let meshes = frame.leaf_meshes();
  let leaf_set = compare_leaf_sets(&mut t, &leaves, &gpu_leaves);
  let watertight = check_mesh_watertight(&mut t, &gpu_leaves, &meshes);
  let cpu_watertight = check_watertight(&mut t, &leaves);
  let error = measure_mesh_error(&t, &gpu_leaves, &meshes, error_resolution);

  let mut max_position_offset: f64 = 0.;
  let mut max_normal_deviation: f64 = 0.;
  for ((leaf, mesh), draw) in gpu_leaves.iter().zip(&meshes).zip(&frame.leaves) {
    let reference = t.leaf_mesh(leaf);
    let item = &items[leaf.key.item as usize];
    let patch = &model.patches[item.patch as usize];
    let flip = patch.flip_normal != model.instances[item.instance as usize].flip_normal;
    for (index, (a, b)) in mesh.positions.iter().zip(&reference.positions).enumerate() {
      // Near threshold edge linearization decisions move a vertex by up to the edge tolerance.
      let offset = (*a - *b).length() * camera.pixel_scale(*b);
      max_position_offset = max_position_offset.max(offset);
      let (u, v) = reference.params[index];
      let (_, du, dv) = item
        .surface
        .eval_with_partials(param_to_f64(u), param_to_f64(v));
      let n = du.cross(dv);
      // Partials tiny against the camera space magnitude (poles, slivers) lose their precision
      // in f32, at poles the normal is noise also on the CPU.
      let (lu, lv) = (du.length(), dv.length());
      if lu.min(lv) > 1e-3 * b.length() && n.length() > 1e-3 * lu * lv {
        let n = n.normalize() * if flip { -1. } else { 1. };
        let gpu = frame.vertices[(draw.vertex_base as usize) + index].normal;
        let gpu = Vec3::new(gpu.x, gpu.y, gpu.z).into_f64();
        max_normal_deviation = max_normal_deviation.max((gpu - n).length());
      }
    }
  }

  FrameComparison {
    cpu_leaves: leaves.len(),
    gpu_leaves: gpu_leaves.len(),
    leaf_set,
    cpu_work_items: t.stats.work_items.clone(),
    gpu_work_items: frame.stats.work_items.clone(),
    cpu_leaves_per_k: t.stats.leaves_per_k.clone(),
    gpu_leaves_per_k: frame.stats.leaves_per_k.to_vec(),
    watertight,
    cpu_watertight,
    error,
    max_position_offset,
    max_normal_deviation,
  }
}

/// The checks every frame must pass.
fn assert_frame(c: &FrameComparison, config: &TessellationConfig, frame: &GpuFrameReadback) {
  assert_eq!(frame.stats.overflow, 0, "{c:#?}");
  assert!(
    c.leaf_set.max_mismatch_margin < 1e-3,
    "mismatch beyond rounding: {c:#?}"
  );
  assert_eq!(c.watertight.mismatched_vertices, 0, "{c:#?}");
  assert_eq!(c.watertight.non_manifold_lines, 0, "{c:#?}");
  assert!(c.watertight.max_t_junction_pixels < 1e-3, "{c:#?}");
  assert!(
    c.watertight.uncovered_vertices <= c.cpu_watertight.uncovered_vertices,
    "{c:#?}"
  );
  assert!(
    c.error.max_pixel_error <= config.pixel_error + 0.02,
    "{c:#?}"
  );
  assert!(c.max_position_offset < 1e-2, "{c:#?}");
  assert!(c.max_normal_deviation < 1e-3, "{c:#?}");
  for (b, args) in frame.draw_args.iter().enumerate() {
    let k = 1u32 << b;
    assert_eq!(args.vertex_count, 6 * k * k);
    assert_eq!(args.instance_count, frame.stats.leaves_per_k[b]);
  }
}

#[pollster::test]
async fn data_upload_matches_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let model = random_model(40, 9, (3., 15.));
  let camera = test_camera();
  let mut t = GpuTessellator::new(&gpu, &model, TessellationConfig::default());
  run_frame(&gpu, &mut t, &camera).await;

  let items = cpu_items(&model, &camera, false);
  let data = &t.active;
  assert_eq!(data.items.len(), items.len());
  let origins = face_origins(&model);
  for (gpu_item, item) in data.items.iter().zip(&items) {
    assert_eq!(
      (gpu_item.instance, gpu_item.patch),
      (item.instance, item.patch)
    );
    let patch = &model.patches[item.patch as usize];
    let relative = relative_to_origin(&patch.surface, origins[patch.face.index()]);
    let offset = data.patch_cp_offset[item.patch as usize] as usize;
    for (i, p) in relative.control_points.iter().enumerate() {
      assert_eq!(data.cp_pool[offset + i], p.into_f32());
    }
    let flip = patch.flip_normal != model.instances[item.instance as usize].flip_normal;
    assert_eq!(gpu_item.flags & ITEM_FLAG_FLIP_NORMAL != 0, flip);
    assert_eq!(
      gpu_item.flags & ITEM_FLAG_TRIMMED != 0,
      !patch.is_untrimmed()
    );
  }

  let mut encoder = gpu.create_encoder();
  let device = &t.device_data;
  let gpu_items = encoder.read_storage_array(&gpu.device, &device.items.clone().into_rw_view());
  let pool = encoder.read_storage_array(&gpu.device, &device.cp_pool.clone().into_rw_view());
  let camera_cps = encoder.read_storage_array(&gpu.device, &device.camera_cps);
  gpu.submit_encoder(encoder);
  assert_eq!(gpu_items.await.unwrap(), data.items);
  assert_eq!(pool.await.unwrap(), data.cp_pool);
  let camera_cps = camera_cps.await.unwrap();
  let mut bits = fast_hash_collection::FastHashMap::default();
  for (gpu_item, item) in data.items.iter().zip(&items) {
    let base = gpu_item.camera_cp_offset as usize;
    for (i, p) in item.surface.control_points.iter().enumerate() {
      let g = camera_cps[base + i].into_f64();
      assert!((g - *p).length() <= 1e-5 * p.length(), "{g:?} {p:?}");
      // Bitwise equal f64 control points of one instance stay bitwise equal in f32.
      let key = [p.x, p.y, p.z, p.w].map(f64::to_bits);
      let gpu_bits = [g.x, g.y, g.z, g.w].map(|x| (x as f32).to_bits());
      assert_eq!(*bits.entry(key).or_insert(gpu_bits), gpu_bits);
    }
  }
}

/// Synthetic scenes at several distances, including near plane crossings, forced leaves at a
/// low `max_level`, and the untrimmed filter.
#[pollster::test]
async fn frames_match_cpu_on_synthetic_scenes() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let default = TessellationConfig::default();
  let cases = [
    (41, 12, (3., 15.), default, false),
    (42, 12, (15., 80.), default, false),
    (43, 8, (1.5, 4.), default, true),
    (
      44,
      6,
      (0.3, 0.8),
      TessellationConfig {
        max_level: 5,
        ..default
      },
      false,
    ),
    (
      45,
      10,
      (2., 10.),
      TessellationConfig {
        pixel_error: 0.2,
        edge_error_share: 0.25,
        k_max: 8,
        ..default
      },
      false,
    ),
  ];
  for (seed, faces, depth, config, untrimmed_only) in cases {
    let model = random_model(seed, faces, depth);
    let camera = test_camera();
    let mut t = GpuTessellator::new(&gpu, &model, config);
    t.set_untrimmed_only(untrimmed_only);
    let frame = run_converged_frame(&gpu, &mut t, &camera).await;
    let c = compare_frame(&model, &camera, config, untrimmed_only, &frame, 4);
    println!(
      "seed {seed}: leaves cpu {} gpu {} (mismatched {}, max margin {:.2e}) work items cpu {:?} gpu {:?} k cpu {:?} gpu {:?} forced {} | T junctions {} max {:.2e} px | error {:.3} px | offset {:.2e} px | rounds {:?}",
      c.cpu_leaves,
      c.gpu_leaves,
      c.leaf_set.mismatched,
      c.leaf_set.max_mismatch_margin,
      c.cpu_work_items,
      c.gpu_work_items,
      c.cpu_leaves_per_k,
      c.gpu_leaves_per_k,
      frame.stats.forced_leaves,
      c.watertight.t_junctions,
      c.watertight.max_t_junction_pixels,
      c.error.max_pixel_error,
      c.max_position_offset,
      frame.stats.requests_per_round,
    );
    assert!(c.gpu_leaves > 0);
    if c.leaf_set.mismatched == 0 {
      let cpu: Vec<_> = c.cpu_work_items.iter().map(|&n| n as u32).collect();
      assert_eq!(cpu, c.gpu_work_items);
      let cpu: Vec<_> = c.cpu_leaves_per_k.iter().map(|&n| n as u32).collect();
      assert_eq!(cpu[..], c.gpu_leaves_per_k[..cpu.len()]);
    }
    assert!(c.watertight.shared_lines > 0, "{c:#?}");
    assert_frame(&c, &config, &frame);
  }
}

/// Tiny capacities: the overflowing frame drops writes (never out of bounds, wgpu validation
/// would fail the test) and reports it, the buffers then grow until the frame fits and matches
/// the unconstrained result.
#[pollster::test]
async fn overflow_grows_buffers() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let model = random_model(46, 10, (1.5, 6.));
  let camera = test_camera();
  let config = TessellationConfig::default();
  let mut reference = GpuTessellator::new(&gpu, &model, config);
  let expected = run_frame(&gpu, &mut reference, &camera).await;
  assert_eq!(expected.stats.overflow, 0);

  let mut t = GpuTessellator::new(&gpu, &model, config);
  t.set_capacity(TessCapacity {
    queue: 8,
    leaves: 8,
    vertices: 64,
    requests: 16,
  });
  let small = t.capacity();
  let first = run_frame(&gpu, &mut t, &camera).await;
  let flags = first.stats.overflow;
  assert_ne!(flags & OVERFLOW_QUEUE, 0, "{:?}", first.stats);
  assert!(first.leaves.len() <= small.leaves as usize);
  let mut frames = 1;
  let last = loop {
    // Let the asynchronous statistics arrive and the buffers grow.
    let mut encoder = gpu.create_encoder();
    t.encode_frame(&gpu, &mut encoder, &camera);
    gpu.submit_encoder(encoder);
    gpu.poll(true);
    let frame = t.read_back(&gpu).await;
    frames += 1;
    if frame.stats.overflow == 0 && t.last_stats().is_some_and(|s| s.overflow == 0) {
      break frame;
    }
    assert!(frames < 40, "no convergence: {:?}", frame.stats);
  };
  println!(
    "overflow flags {flags:#b} on {small:?}, converged after {frames} frames to {:?}",
    t.capacity()
  );
  let key = |f: &GpuFrameReadback| {
    let mut leaves: Vec<_> = f
      .cpu_leaves()
      .iter()
      .map(|l| (l.key.item, l.key.level, l.key.ix, l.key.iy, l.k))
      .collect();
    leaves.sort();
    leaves
  };
  assert_eq!(key(&last), key(&expected));
}

fn print_stats(label: &str, s: &GpuTessellationStats) {
  println!(
    "{label}: {} items ({} patches rejected) | {} leaves {} triangles {} vertices | work items {:?} | leaves per level {:?} | k {:?} | culled {} splits {:?} forced {} edge raised {} | requests per round {:?} | overflow {:#b} | {} dispatches",
    s.items,
    s.rejected_patches,
    s.leaves,
    s.triangles,
    s.vertices,
    s.work_items,
    s.leaves_per_level,
    s.leaves_per_k,
    s.culled,
    s.splits,
    s.forced_leaves,
    s.edge_raised_leaves,
    s.requests_per_round,
    s.overflow,
    s.dispatches,
  );
}

/// GPU against CPU on the steam controller model (skipped when the file is missing), prints
/// the statistics and the frame time: `cargo test --release -p rendiation-parametric-rendering
/// steam -- --ignored --nocapture`.
#[pollster::test]
#[ignore]
async fn steam_controller_gpu_vs_cpu() {
  let Ok(model) = ParametricNormalizedModel::load_pnm(STEAM_MODEL) else {
    println!("{STEAM_MODEL} not found, skipped");
    return;
  };
  let gpu = GPU::new(Default::default()).await.unwrap();
  let config = TessellationConfig::default();
  for (zoom, untrimmed_only) in [
    (1., false),
    (4., false),
    (4., true),
    (20., false),
    (80., false),
  ] {
    let camera = fit_camera(&model, zoom);
    let mut t = GpuTessellator::new(&gpu, &model, config);
    t.set_untrimmed_only(untrimmed_only);
    // Grow the buffers to fit, then time a few frames (submit and wait).
    let mut frame = run_frame(&gpu, &mut t, &camera).await;
    for _ in 0..8 {
      if frame.stats.overflow == 0 {
        break;
      }
      let mut encoder = gpu.create_encoder();
      t.encode_frame(&gpu, &mut encoder, &camera);
      gpu.submit_encoder(encoder);
      gpu.poll(true);
      frame = run_frame(&gpu, &mut t, &camera).await;
    }
    let mut times = Vec::new();
    let mut encode_times = Vec::new();
    for _ in 0..5 {
      let start = Instant::now();
      let mut encoder = gpu.create_encoder();
      t.encode_frame(&gpu, &mut encoder, &camera);
      encode_times.push(start.elapsed().as_secs_f64() * 1e3);
      gpu.submit_encoder(encoder);
      gpu.poll(true);
      times.push(start.elapsed().as_secs_f64() * 1e3);
    }
    let frame_after = t.read_back(&gpu).await;
    let label = format!("fit:{zoom} untrimmed only {untrimmed_only}");
    print_stats(&label, &frame_after.stats);
    let start = Instant::now();
    let c = compare_frame(&model, &camera, config, untrimmed_only, &frame_after, 2);
    println!(
      "  leaves cpu {} gpu {} | mismatched {} (max margin {:.2e}) | work items cpu {:?} | k cpu {:?} | watertight: shared lines {} open {} non manifold {} shared vertices {} mismatched {} T junctions {} max offset {:.2e} px ({:.2e} relative) uncovered {} | max error {:.4} px over {} samples ({} over) | max position offset {:.2e} px | normal deviation {:.2e} | frame {:.2?} ms (encode + submit + wait), encode {:.2?} ms | check {:.1} s",
      c.cpu_leaves,
      c.gpu_leaves,
      c.leaf_set.mismatched,
      c.leaf_set.max_mismatch_margin,
      c.cpu_work_items,
      c.cpu_leaves_per_k,
      c.watertight.shared_lines,
      c.watertight.open_lines,
      c.watertight.non_manifold_lines,
      c.watertight.shared_vertices,
      c.watertight.mismatched_vertices,
      c.watertight.t_junctions,
      c.watertight.max_t_junction_pixels,
      c.watertight.max_t_junction_relative,
      c.watertight.uncovered_vertices,
      c.error.max_pixel_error,
      c.error.samples,
      c.error.samples_over_threshold,
      c.max_position_offset,
      c.max_normal_deviation,
      times,
      encode_times,
      start.elapsed().as_secs_f64(),
    );
    for m in &c.watertight.mismatches {
      let items = cpu_items(&model, &camera, untrimmed_only);
      let face = |i: u32| {
        let item = &items[i as usize];
        (item.instance, model.patches[item.patch as usize].face)
      };
      println!(
        "  mismatch {m:?} instance and face {:?} {:?}",
        face(m.a.0),
        face(m.b.0)
      );
    }
    assert_eq!(c.watertight.mismatched_vertices, 0);
    assert!(c.leaf_set.max_mismatch_margin < 1e-2);
  }
}
