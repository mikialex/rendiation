//! View dependent tessellation output (`convert ... --camera`): runs the CPU reference of the
//! adaptive GPU tessellation (`parametric-tessellation`), prints its statistics and checks, and
//! writes the leaves as a vertex colored glTF together with the camera.

use std::path::Path;
use std::time::Instant;

use parametric_normalized_model::*;
use parametric_tessellation::verify::*;
use parametric_tessellation::*;
use rendiation_algebra::*;

use crate::gltf::{GltfDoc, write_glb};
use crate::mesh::MeshData;

#[derive(Debug, Clone, Copy)]
pub enum CameraPlacement {
  /// Fit the model bounds, looking from a fixed diagonal direction, with the distance divided
  /// by `zoom`.
  Fit {
    zoom: f64,
  },
  LookAt {
    eye: Vec3<f64>,
    target: Vec3<f64>,
  },
}

#[derive(Debug, Clone, Copy)]
pub struct ViewOptions {
  pub placement: CameraPlacement,
  pub viewport: Vec2<f64>,
  pub fov_y_degrees: f64,
  pub config: TessellationConfig,
  pub untrimmed_only: bool,
  /// Run the error and edge consistency checks (slow for big models).
  pub verify: bool,
}

impl Default for ViewOptions {
  fn default() -> Self {
    Self {
      placement: CameraPlacement::Fit { zoom: 1. },
      viewport: Vec2::new(1920., 1080.),
      fov_y_degrees: 45.,
      config: TessellationConfig::default(),
      untrimmed_only: false,
      verify: false,
    }
  }
}

pub fn write_view_dependent_glb(
  model: &ParametricNormalizedModel,
  path: &Path,
  options: &ViewOptions,
) -> Result<(), Box<dyn std::error::Error>> {
  let camera = make_camera(model, options);
  let (items, rejected) = camera_space_items(model, &camera, |p| {
    !options.untrimmed_only || p.is_untrimmed()
  });
  let start = Instant::now();
  let mut t = Tessellator::new(&items, camera, options.config);
  let leaves = t.run();
  let elapsed = start.elapsed();
  print_stats(&t, &leaves, rejected, elapsed.as_secs_f64());

  let uniform = uniform_triangle_count(&items, &camera, &options.config, 1024);
  println!(
    "  uniform per patch grid: {} patches {} triangles ({:.2}x adaptive), {} clamped to 1024",
    uniform.patches,
    uniform.triangles,
    uniform.triangles as f64 / t.stats.triangles.max(1) as f64,
    uniform.clamped
  );

  if options.verify {
    let start = Instant::now();
    let error = measure_error(&mut t, &leaves, 3);
    println!(
      "  error: max {:.4} px over {} samples, {} over {} px ({:.1} s)",
      error.max_pixel_error,
      error.samples,
      error.samples_over_threshold,
      options.config.pixel_error,
      start.elapsed().as_secs_f64()
    );
    let w = check_watertight(&mut t, &leaves);
    println!(
      "  boundary vertices by nested edge interpolations: {:?}",
      t.stats.position_chains
    );
    println!(
      "  edges: lines shared {} open {} non manifold {} | shared vertices {} mismatched {} | T junctions {} max offset {:.2e} px ({:.2e} relative) | uncovered {}",
      w.shared_lines,
      w.open_lines,
      w.non_manifold_lines,
      w.shared_vertices,
      w.mismatched_vertices,
      w.t_junctions,
      w.max_t_junction_pixels,
      w.max_t_junction_relative,
      w.uncovered_vertices
    );
  }

  let mut doc = GltfDoc::new();
  let mesh = leaves_mesh(model, &mut t, &leaves);
  doc.add_vertex_colored_mesh(&mesh.0, &mesh.1, Mat4::identity());
  doc.add_perspective_camera(
    camera.camera_to_world().into_f32(),
    camera.fov_y() as f32,
    (camera.viewport.x / camera.viewport.y) as f32,
    camera.near as f32,
  );
  let (root, bin) = doc.into_root();
  write_glb(path, &root, &bin)?;
  Ok(())
}

fn make_camera(model: &ParametricNormalizedModel, options: &ViewOptions) -> Camera {
  let fov_y = options.fov_y_degrees.to_radians();
  let (eye, target) = match options.placement {
    CameraPlacement::LookAt { eye, target } => (eye, target),
    CameraPlacement::Fit { zoom } => {
      let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
      let mut max = min * -1.;
      for instance in &model.instances {
        for patch in &model.patches[instance.patches.clone()] {
          for p in &patch.surface.control_points {
            let p = (instance.transform * *p).to_euclid();
            min = min.min(p);
            max = max.max(p);
          }
        }
      }
      let center = (min + max) * 0.5;
      let radius = ((max - min).length() * 0.5).max(1e-9);
      let distance = radius / (fov_y * 0.5).sin() / zoom;
      let direction = Vec3::new(1., 0.8, 1.3).normalize();
      (center + direction * distance, center)
    }
  };
  let distance = (eye - target).length();
  Camera::look_at(
    eye,
    target,
    Vec3::new(0., 1., 0.),
    fov_y,
    options.viewport,
    distance * 1e-3,
  )
}

fn print_stats(t: &Tessellator, leaves: &[Leaf], rejected: usize, seconds: f64) {
  let s = &t.stats;
  let c = t.config();
  println!(
    "view dependent tessellation: {} items ({} rejected for degree) | {} leaves {} triangles | {:.1} ms",
    t.items().len(),
    rejected,
    leaves.len(),
    s.triangles,
    seconds * 1e3
  );
  println!(
    "  config: {} px (edge share {}), max triangle {} px, k_max {}, max level {}",
    c.pixel_error, c.edge_error_share, c.max_triangle_size, c.k_max, c.max_level
  );
  println!("  work items per level: {:?}", s.work_items);
  println!("  leaves per level: {:?}", s.leaves_per_level);
  let ks: Vec<_> = s
    .leaves_per_k
    .iter()
    .enumerate()
    .map(|(i, n)| format!("k{}: {n}", 1 << i))
    .collect();
  println!("  leaves per k: {}", ks.join(", "));
  println!(
    "  k raised for edge break points: {} leaves, {} triangles without the raise",
    s.edge_raised_leaves, s.triangles_without_edge_raise
  );
  println!(
    "  culled {} | splits near {} interior {} size {} edge {} | forced leaves {} forced edge segments {} | edge tests {}",
    s.culled,
    s.splits[SplitReason::NearPlane as usize],
    s.splits[SplitReason::Interior as usize],
    s.splits[SplitReason::Size as usize],
    s.splits[SplitReason::Edge as usize],
    s.forced_leaves,
    s.forced_edge_segments,
    s.edge_tests
  );
}

/// All leaves in world space, colored by k (brightness varied per leaf), trimmed patches
/// (drawn without their trim) washed out.
fn leaves_mesh(
  model: &ParametricNormalizedModel,
  t: &mut Tessellator,
  leaves: &[Leaf],
) -> (MeshData, Vec<Vec4<f32>>) {
  const PALETTE: [[f32; 3]; 5] = [
    [0.85, 0.85, 0.85],
    [0.35, 0.65, 0.95],
    [0.35, 0.85, 0.45],
    [0.95, 0.80, 0.30],
    [0.95, 0.40, 0.35],
  ];
  let camera_to_world = t.camera().camera_to_world();
  let mut mesh = MeshData {
    positions: Vec::new(),
    normals: Vec::new(),
    uvs: Vec::new(),
    indices: Vec::new(),
  };
  let mut colors = Vec::new();
  for (leaf_index, leaf) in leaves.iter().enumerate() {
    let item = &t.items()[leaf.key.item as usize];
    let patch = &model.patches[item.patch as usize];
    let flip = patch.flip_normal != model.instances[item.instance as usize].flip_normal;
    let surface = item.surface.clone();

    let base = PALETTE[(leaf.k.trailing_zeros() as usize).min(PALETTE.len() - 1)];
    let hash = (leaf_index as u32).wrapping_mul(0x9E37_79B9) >> 24;
    let shade = 0.75 + 0.25 * hash as f32 / 255.;
    let mut color = base.map(|c| c * shade);
    if !patch.is_untrimmed() {
      color = color.map(|c| c * 0.4 + 0.45);
    }
    let color = Vec4::new(color[0], color[1], color[2], 1.);

    let leaf_mesh = t.leaf_mesh(leaf);
    let offset = mesh.positions.len() as u32;
    for (&(u, v), &p) in leaf_mesh.params.iter().zip(&leaf_mesh.positions) {
      let (_, du, dv) = surface.eval_with_partials(param_to_f64(u), param_to_f64(v));
      let n = du.cross(dv);
      let n = if n.length() > 0. { n.normalize() } else { n };
      let n = if flip { n * -1. } else { n };
      let n = camera_to_world * Vec4::new(n.x, n.y, n.z, 0.);
      mesh.positions.push((camera_to_world * p).into_f32());
      mesh.normals.push(Vec3::new(n.x, n.y, n.z).into_f32());
      colors.push(color);
    }
    mesh.indices.extend(leaf_mesh.triangles().map(|[a, b, c]| {
      let tri = [a + offset, b + offset, c + offset];
      if flip { [tri[0], tri[2], tri[1]] } else { tri }
    }));
  }
  (mesh, colors)
}
