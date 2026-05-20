mod curve_convert;
mod parameter_remapping;
mod surface_convert;
mod topology;

pub use curve_convert::{
  convert_90deg_circular_arc_to_bezier, convert_bspline_curve_to_bezier,
  convert_circle_to_bezier_curves, convert_ellipse_to_bezier_curves, convert_line_to_bezier,
  convert_rational_bspline_curve_to_bezier,
};
use rendiation_algebra::{Vec2, Vec3};
use rendiation_step_reader::table::Table;
use surface_convert::convert_any_surface_to_bezier_patches;
pub use surface_convert::{
  convert_bezier_surface_to_patch, convert_bspline_surface_to_bezier_patches,
  convert_cone_to_bezier_patches, convert_cylinder_to_bezier_patches,
  convert_extrusion_surface_to_bezier_patches, convert_plane_to_bezier_patch,
  convert_rational_bspline_surface_to_bezier_patches, convert_sphere_to_bezier_patches,
  convert_torus_to_bezier_patches,
};
use topology::{collect_face_surface_data, EdgeData};

use crate::step::curve_convert::{convert_any_curve_to_bezier, extract_2d_curve_points};
use crate::step::parameter_remapping::{
  connect_polylines_with_boundary, point_in_polygon, remap_2d_points_to_patch,
};
use crate::step::surface_convert::{OriginalSurface, PatchParamRange};
use crate::surface_trim::QuadraticBezierCurve2d;
use crate::*;

// ── Debug logging ─────────────────────────────────────────────────────

/// Set to `true` to enable verbose step-pipeline tracing via `eprintln!`.
pub const STEP_DEBUG_LOG: bool = true;

/// Log a debug message when STEP_DEBUG_LOG is enabled.
/// Uses `format_args!`-style formatting call: `step_debug(format_args!(...))`.
#[allow(unused)]
pub(crate) fn step_debug(args: std::fmt::Arguments<'_>) {
  if STEP_DEBUG_LOG {
    eprintln!("{}", args);
  }
}

/// Convenience: `step_dbg!(format_string, ...)`
macro_rules! step_dbg {
  ($($arg:tt)*) => {
    $crate::step::step_debug(format_args!($($arg)*))
  };
}
pub(crate) use step_dbg;

// ── Configuration ─────────────────────────────────────────────────────

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

// ── Error ─────────────────────────────────────────────────────────────

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

fn assemble_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  let face_data_list = collect_face_surface_data(table)?;

  step_dbg!("step: {} faces collected", face_data_list.len());

  let mut trimmed_surfaces: Vec<TrimmedSurface> = Vec::new();
  let mut curves_3d: Vec<RationalBezierCurve3d<f32>> = Vec::new();

  for (fi, face_data) in face_data_list.iter().enumerate() {
    let (patches, original_surface) =
      match convert_any_surface_to_bezier_patches(&face_data.surface) {
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

    for (pi, patch) in patches.iter().enumerate() {
      match &patch_trim_boundaries[pi] {
        Some(trim) => {
          trimmed_surfaces.push(TrimmedSurface {
            surface: patch.surface.clone(),
            trim_boundary: trim.clone(),
          });
        }
        None => {
          step_dbg!("step: face #{fi} patch #{pi} discarded (fully trimmed)");
        }
      }
    }

    for edge in &face_data.edges {
      match convert_any_curve_to_bezier(&edge.curve_3d) {
        Ok(beziers) => curves_3d.extend(beziers),
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
  patches: &[PatchParamRange],
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
    for curve in beziers {
      let pts = crate::surface_trim::bezier_curve_tessellate::adaptive_tessellate_bezier_curve(
        curve,
        config.tessellate_tolerance,
      );
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
        if u_global >= patch.u_range.0
          && u_global <= patch.u_range.1
          && v_global >= patch.v_range.0
          && v_global <= patch.v_range.1
        {
          let du = patch.u_range.1 - patch.u_range.0;
          let dv = patch.v_range.1 - patch.v_range.0;
          if du > 0.0 && dv > 0.0 {
            let u_local = ((u_global - patch.u_range.0) / du).clamp(0.0, 1.0);
            let v_local = ((v_global - patch.v_range.0) / dv).clamp(0.0, 1.0);
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

      let components = connect_polylines_with_boundary(polylines, boundary_epsilon);
      let mut quadratics = Vec::new();
      for component in &components {
        if component.len() >= 2 {
          let qs = crate::surface_trim::reconstruct_boundary(component, config.fit_tolerance);
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
  patches: &[PatchParamRange],
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
    u_min = u_min.min(p.u_range.0);
    u_max = u_max.max(p.u_range.1);
    v_min = v_min.min(p.v_range.0);
    v_max = v_max.max(p.v_range.1);
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
          let u_global =
            patches[pi].u_range.0 + p_local.x * (patches[pi].u_range.1 - patches[pi].u_range.0);
          let v_global =
            patches[pi].v_range.0 + p_local.y * (patches[pi].v_range.1 - patches[pi].v_range.0);
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
    let cu = (patches[pi].u_range.0 + patches[pi].u_range.1) * 0.5;
    let cv = (patches[pi].v_range.0 + patches[pi].v_range.1) * 0.5;
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
  patches: &[PatchParamRange],
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
      let remapped = remap_2d_points_to_patch(&points_2d, patch);
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
