mod curve2d_sample;
mod curve3d_convert;
mod helpers;
mod parameter_remapping;
mod placement;
mod surface_convert;
mod surface_project;
mod topology;

use curve2d_sample::*;
use curve3d_convert::*;
use helpers::*;
use placement::*;
use rendiation_step_reader::entities::SurfaceAny;
use rendiation_step_reader::table::Table;
use surface_convert::*;
use surface_project::*;
use topology::*;

use crate::step::parameter_remapping::{
  connect_polylines_with_boundary, point_in_polygon, remap_2d_points_to_patch,
};
use crate::*;

/// Set to `true` to enable verbose step-pipeline tracing via `println!`.
pub const STEP_DEBUG_LOG: bool = true;

/// Log a debug message when STEP_DEBUG_LOG is enabled.
/// Uses `format_args!`-style formatting call: `step_debug(format_args!(...))`.
#[allow(unused)]
pub(crate) fn step_debug(args: std::fmt::Arguments<'_>) {
  if STEP_DEBUG_LOG {
    println!("{}", args);
  }
}

/// Convenience: `step_dbg!(format_string, ...)`
macro_rules! step_dbg {
  ($($arg:tt)*) => {
    $crate::step::step_debug(format_args!($($arg)*))
  };
}
pub(crate) use step_dbg;

#[derive(Debug, Clone)]
pub struct StepReadConfig {
  pub tessellate_tolerance: f32,
  pub project_grid: usize,
  pub project_tolerance: f32,
  pub project_max_iter: usize,
  pub fit_tolerance: f32,
}

impl Default for StepReadConfig {
  fn default() -> Self {
    Self {
      tessellate_tolerance: 1e-4,
      project_grid: 8,
      project_tolerance: 1e-6,
      project_max_iter: 20,
      fit_tolerance: 1e-4,
    }
  }
}

#[derive(Debug)]
pub enum StepReadError {
  ParseError(String),
  MissingEntity { entity_type: &'static str, id: u64 },
  UnsupportedSurface { entity_type: &'static str, id: u64 },
  UnsupportedCurve { entity_type: &'static str, id: u64 },
  ConversionError(String),
}

impl std::fmt::Display for StepReadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StepReadError::ParseError(msg) => write!(f, "STEP parse error: {msg}"),
      StepReadError::MissingEntity { entity_type, id } => {
        write!(f, "Missing entity {entity_type} #{id}")
      }
      StepReadError::UnsupportedSurface { entity_type, id } => {
        write!(f, "Unsupported surface type {entity_type} (#{id})")
      }
      StepReadError::UnsupportedCurve { entity_type, id } => {
        write!(f, "Unsupported curve type {entity_type} (#{id})")
      }
      StepReadError::ConversionError(msg) => write!(f, "Conversion error: {msg}"),
    }
  }
}

impl std::error::Error for StepReadError {}

// ── Main entry ────────────────────────────────────────────────────────

pub fn read_parametric_rendering_data_from_step(
  step_str: &str,
  config: StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  let table =
    Table::from_step(step_str).ok_or_else(|| StepReadError::ParseError("Parse failed".into()))?;

  assemble_from_table(&table, &config)
}

pub fn read_parametric_rendering_data_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  assemble_from_table(table, config)
}

fn transform_homogeneous_point(cp: &mut Vec4<f32>, placement: &Placement) {
  let w = cp.w;
  if w.abs() < 1e-12 {
    return;
  }
  let (origin, x_dir, y_dir, z_dir) = *placement;
  let p = Vec3::new(cp.x / w, cp.y / w, cp.z / w);
  let tp = origin + x_dir * p.x + y_dir * p.y + z_dir * p.z;
  cp.x = tp.x * w;
  cp.y = tp.y * w;
  cp.z = tp.z * w;
}

fn apply_placement_to_surface(
  surface: &mut crate::surface::RationalBezierSurface<f32>,
  placement: &Placement,
) {
  for cp in surface.control_points_mut() {
    transform_homogeneous_point(cp, placement);
  }
}

fn apply_placement_to_curve(curve: &mut RationalBezierCurve3d<f32>, placement: &Placement) {
  for cp in curve.control_points_mut() {
    transform_homogeneous_point(cp, placement);
  }
}

/// Compute the U,V extent of a plane face by projecting edge curve points
/// onto the plane's local coordinate system.
///
/// Returns (u_min, u_max, v_min, v_max).
/// Falls back to (-1, 1, -1, 1) if no points can be obtained.
fn compute_plane_face_extent(
  position: &rendiation_step_reader::entities::Axis2Placement3d,
  edges: &[EdgeData],
  config: &StepReadConfig,
) -> (f32, f32, f32, f32) {
  let origin = cartesian_point_to_vec3(&position.location);
  let normal = direction_to_vec3(&position.axis);
  let u_dir = axis2_x_dir(position);
  let v_dir = normal.cross(u_dir).normalize();

  let mut u_min = f32::MAX;
  let mut u_max = f32::MIN;
  let mut v_min = f32::MAX;
  let mut v_max = f32::MIN;
  let mut any_point = false;

  for edge in edges {
    let beziers = match convert_any_curve_to_bezier(&edge.curve_3d) {
      Ok(b) => b,
      Err(_) => continue,
    };
    for curve in &beziers {
      let pts = adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance);
      for p in &pts {
        let local = *p - origin;
        let u = local.dot(u_dir);
        let v = local.dot(v_dir);
        u_min = u_min.min(u);
        u_max = u_max.max(u);
        v_min = v_min.min(v);
        v_max = v_max.max(v);
        any_point = true;
      }
    }
  }

  if !any_point {
    return (-1.0, 1.0, -1.0, 1.0);
  }

  (u_min, u_max, v_min, v_max)
}

/// Compute the V-axis extent for cylinder/cone faces by projecting edge
/// curve points onto the surface's axis direction.
///
/// V is the signed distance from the placement origin along the axis.
/// Returns (v_min, v_max). Falls back to (0.0, 1.0)
/// if no points can be obtained.
fn compute_axis_v_extent(
  position: &rendiation_step_reader::entities::Axis2Placement3d,
  edges: &[EdgeData],
  config: &StepReadConfig,
) -> (f64, f64) {
  let origin = cartesian_point_to_vec3(&position.location);
  let axis = direction_to_vec3(&position.axis).normalize();

  let mut v_min = f64::MAX;
  let mut v_max = f64::MIN;
  let mut any_point = false;

  for edge in edges {
    let beziers = match convert_any_curve_to_bezier(&edge.curve_3d) {
      Ok(b) => b,
      Err(_) => continue,
    };
    for curve in &beziers {
      let pts = adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance);
      for p in &pts {
        let v = (*p - origin).dot(axis) as f64;
        v_min = v_min.min(v);
        v_max = v_max.max(v);
        any_point = true;
      }
    }
  }

  if !any_point {
    return (0.0, 1.0);
  }

  (v_min, v_max)
}

fn assemble_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  let face_data_list = collect_face_surface_data(table)?;

  step_dbg!("step: {} faces collected", face_data_list.len());

  let mut trimmed_surfaces: Vec<TrimmedSurface> = Vec::new();
  let mut curves_3d: Vec<RationalBezierCurve3d<f32>> = Vec::new();

  for (fi, face_data) in face_data_list.iter().enumerate() {
    let plane_extent = if let SurfaceAny::Plane(ref p) = &face_data.surface {
      Some(compute_plane_face_extent(
        &p.position,
        &face_data.edges,
        config,
      ))
    } else {
      None
    };

    let v_range = match &face_data.surface {
      SurfaceAny::CylindricalSurface(c) => {
        Some(compute_axis_v_extent(&c.position, &face_data.edges, config))
      }
      SurfaceAny::ConicalSurface(c) => {
        Some(compute_axis_v_extent(&c.position, &face_data.edges, config))
      }
      _ => None,
    };

    let (mut patches, original_surface) = match convert_and_split_any_surface_to_bezier_patches(
      &face_data.surface,
      plane_extent,
      v_range,
    ) {
      Ok(p) => p,
      Err(e) => {
        step_dbg!("step: face #{fi} surface conversion failed: {e}");
        continue;
      }
    };

    step_dbg!(
      "step: face #{fi} → {} patches, {} edges",
      patches.len(),
      face_data.edges.len()
    );

    let patch_trim_boundaries =
      process_trim_curves_for_face(&original_surface, &patches, &face_data.edges, config, table);

    if let Some(placement) = face_data.placement {
      let (origin, x_dir, _y_dir, z_dir) = placement;
      step_dbg!(
        "step: face #{fi} placement: origin=({:.3},{:.3},{:.3}) x=({:.3},{:.3},{:.3}) z=({:.3},{:.3},{:.3})",
        origin.x, origin.y, origin.z,
        x_dir.x, x_dir.y, x_dir.z,
        z_dir.x, z_dir.y, z_dir.z,
      );
      for patch in &mut patches {
        apply_placement_to_surface(&mut patch.surface, &placement);
      }
    } else {
      step_dbg!("step: face #{fi} no placement");
    }

    // Surface position debug (first two faces only)
    if fi < 2 && !patches.is_empty() {
      let p0 = patches[0].surface.evaluate(0.5, 0.5);
      step_dbg!(
        "step: face #{fi} patch center: ({:.3},{:.3},{:.3})",
        p0.x,
        p0.y,
        p0.z
      );
    }

    for (pi, patch) in patches.iter().enumerate() {
      match &patch_trim_boundaries[pi] {
        Some(trim) => {
          trimmed_surfaces.push(TrimmedSurface {
            surface: patch.surface.clone(),
            trim_boundary: trim.clone(),
            is_back_face: face_data.is_back_face,
          });
        }
        None => {
          step_dbg!("step: face #{fi} patch #{pi} discarded (fully trimmed)");
        }
      }
    }

    for edge in &face_data.edges {
      match convert_any_curve_to_bezier(&edge.curve_3d) {
        Ok(mut beziers) => {
          if let Some(placement) = face_data.placement {
            for curve in &mut beziers {
              apply_placement_to_curve(curve, &placement);
            }
          }
          curves_3d.extend(beziers);
        }
        Err(e) => {
          step_dbg!("step: convert_any_curve_to_bezier failed: {e:?}");
        }
      }
    }
  }

  step_dbg!(
    "step: done — {} trimmed surfaces, {} 3d curves",
    trimmed_surfaces.len(),
    curves_3d.len()
  );
  Ok(ParametricRenderingData {
    surfaces: trimmed_surfaces,
    curves_3d,
  })
}

/// Project 3D tessellation points onto the original surface once, then
/// distribute the resulting (u,v) coordinates to the correct Bezier patch
/// by checking each patch's parameter range.
///
/// Returns `Some(trim_boundary)` per patch to keep, or `None` for patches
/// that are fully outside the trimmed region and should be discarded.
fn process_trim_curves_for_face(
  original_surface: &OriginalSurface,
  patches: &[SurfaceSubPatch],
  edges: &[EdgeData],
  config: &StepReadConfig,
  table: &Table,
) -> Vec<Option<Vec<QuadraticBezierCurve2d<f32>>>> {
  let n_patches = patches.len();
  if n_patches == 0 || edges.is_empty() {
    return std::iter::repeat_with(|| Some(Vec::new()))
      .take(n_patches)
      .collect();
  }

  let n_edges = edges.len();
  let mut per_patch_polylines: Vec<Vec<Vec<Vec2<f32>>>> =
    vec![vec![Vec::new(); n_edges]; n_patches];

  let proj_dist_tolerance = config.project_tolerance * 10.0;

  for (ei, edge) in edges.iter().enumerate() {
    // Try pcurve path first
    if try_pcurve_for_all_patches(ei, edge, patches, table, &mut per_patch_polylines) {
      continue;
    }

    // Numerical path: tessellate edge once, project each 3D point onto
    // the original surface, then find the correct patch by range check.
    let beziers = match convert_any_curve_to_bezier(&edge.curve_3d) {
      Ok(b) => b,
      Err(_) => continue,
    };

    let mut all_3d_points: Vec<Vec3<f32>> = Vec::new();
    for curve in &beziers {
      let pts = adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance);
      all_3d_points.extend(pts);
    }

    if all_3d_points.is_empty() {
      continue;
    }

    for &p3 in &all_3d_points {
      // Project onto the original surface once
      let Some((u_global, v_global, dist)) = original_surface.project_point(
        p3,
        config.project_grid,
        config.project_tolerance,
        config.project_max_iter,
      ) else {
        continue;
      };

      if dist > proj_dist_tolerance {
        continue;
      }

      // Find which patch contains this (u, v) — O(P) float comparisons
      for (pi, patch) in patches.iter().enumerate() {
        let range = patch.sub_range;
        if u_global >= range.u_range.0
          && u_global <= range.u_range.1
          && v_global >= range.v_range.0
          && v_global <= range.v_range.1
        {
          let du = range.u_range.1 - range.u_range.0;
          let dv = range.v_range.1 - range.v_range.0;
          if du > 0.0 && dv > 0.0 {
            let u_local = ((u_global - range.u_range.0) / du).clamp(0.0, 1.0);
            let v_local = ((v_global - range.v_range.0) / dv).clamp(0.0, 1.0);
            per_patch_polylines[pi][ei].push(Vec2::new(u_local, v_local));
          }
          break;
        }
      }
    }
  }

  // Phase 2: classify empty patches using a global boundary built from
  // non-empty patches, then reconstruct boundaries for kept patches.
  let boundary_epsilon = config.fit_tolerance * 10.0;

  let keep_mask = classify_empty_patches(patches, &per_patch_polylines, boundary_epsilon);

  per_patch_polylines
    .into_iter()
    .enumerate()
    .map(|(pi, edge_polys)| {
      if !keep_mask[pi] {
        return None; // patch fully outside trimmed region
      }
      let polylines: Vec<Vec<Vec2<f32>>> =
        edge_polys.into_iter().filter(|p| !p.is_empty()).collect();

      if polylines.is_empty() {
        return Some(Vec::new()); // patch fully inside — no trim needed
      }

      // Each edge polyline is already tangent-continuous; fit independently
      // so that sharp corners at edge junctions are preserved.
      let mut quadratics = Vec::new();
      for polyline in &polylines {
        if polyline.len() >= 2 {
          let qs = crate::surface_trim::reconstruct_boundary(polyline, config.fit_tolerance);
          quadratics.extend(qs);
        }
      }
      Some(quadratics)
    })
    .collect()
}

/// Build a global trim boundary from non-empty patches and classify empty
/// patches as inside (keep) or outside (discard) the trimmed region.
///
/// Returns a bool per patch: `true` = keep, `false` = discard.
fn classify_empty_patches(
  patches: &[SurfaceSubPatch],
  per_patch_polylines: &[Vec<Vec<Vec2<f32>>>],
  boundary_epsilon: f32,
) -> Vec<bool> {
  let n = patches.len();

  // If all patches are empty (no trim data), keep all — untrimmed face.
  // If all patches are non-empty, keep all — normal case.
  let empty_count = per_patch_polylines
    .iter()
    .filter(|eps| eps.iter().all(|p| p.is_empty()))
    .count();
  if empty_count == 0 || empty_count == n {
    return vec![true; n];
  }

  // Compute global parameter range
  let mut u_min = f32::MAX;
  let mut u_max = f32::MIN;
  let mut v_min = f32::MAX;
  let mut v_max = f32::MIN;
  for p in patches {
    u_min = u_min.min(p.sub_range.u_range.0);
    u_max = u_max.max(p.sub_range.u_range.1);
    v_min = v_min.min(p.sub_range.v_range.0);
    v_max = v_max.max(p.sub_range.v_range.1);
  }
  let du = u_max - u_min;
  let dv = v_max - v_min;
  if du < 1e-10 || dv < 1e-10 {
    return vec![true; n];
  }

  // Collect all polylines from non-empty patches, mapped to
  // normalized global [0,1]² space
  let mut global_polylines: Vec<Vec<Vec2<f32>>> = Vec::new();
  for (pi, edge_polys) in per_patch_polylines.iter().enumerate() {
    for poly in edge_polys {
      if poly.is_empty() {
        continue;
      }
      let mapped: Vec<Vec2<f32>> = poly
        .iter()
        .map(|&p_local| {
          let u_global = patches[pi].sub_range.u_range.0
            + p_local.x * (patches[pi].sub_range.u_range.1 - patches[pi].sub_range.u_range.0);
          let v_global = patches[pi].sub_range.v_range.0
            + p_local.y * (patches[pi].sub_range.v_range.1 - patches[pi].sub_range.v_range.0);
          Vec2::new((u_global - u_min) / du, (v_global - v_min) / dv)
        })
        .collect();
      global_polylines.push(mapped);
    }
  }

  if global_polylines.is_empty() {
    return vec![true; n];
  }

  // Build closed global boundary polygon(s)
  let components = connect_polylines_with_boundary(global_polylines, boundary_epsilon);

  // For each empty patch, test if its center is inside the global boundary
  let mut keep = vec![true; n];
  for (pi, edge_polys) in per_patch_polylines.iter().enumerate() {
    if edge_polys.iter().any(|p| !p.is_empty()) {
      continue; // non-empty patch → already classified as keep
    }

    // Map patch center to normalized global space
    let cu = (patches[pi].sub_range.u_range.0 + patches[pi].sub_range.u_range.1) * 0.5;
    let cv = (patches[pi].sub_range.v_range.0 + patches[pi].sub_range.v_range.1) * 0.5;
    let test_pt = Vec2::new((cu - u_min) / du, (cv - v_min) / dv);

    // Point-in-polygon test against each closed component.
    // If inside any component, keep the patch; otherwise discard.
    let inside = components
      .iter()
      .any(|comp| point_in_polygon(test_pt, comp));
    keep[pi] = inside;
  }

  keep
}

/// Try pcurve path for all patches at once (remaps pcurve points to each patch).
/// Returns true if the pcurve path was used successfully.
fn try_pcurve_for_all_patches(
  ei: usize,
  edge: &EdgeData,
  patches: &[SurfaceSubPatch],
  table: &Table,
  per_patch_polylines: &mut [Vec<Vec<Vec2<f32>>>],
) -> bool {
  if edge.pcurve_entity_ids.is_empty() {
    return false;
  }

  for &entity_id in &edge.pcurve_entity_ids {
    let points_2d = match extract_2d_curve_points(table, entity_id) {
      Ok(p) => p,
      Err(_) => continue,
    };
    if points_2d.is_empty() {
      continue;
    }

    let mut any_mapped = false;
    for (pi, patch) in patches.iter().enumerate() {
      let remapped = remap_2d_points_to_patch(&points_2d, &patch.sub_range);
      if !remapped.is_empty() {
        per_patch_polylines[pi][ei] = remapped;
        any_mapped = true;
      }
    }
    if any_mapped {
      return true;
    }
  }
  false
}
