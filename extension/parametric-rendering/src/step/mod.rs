mod curve_convert;
mod parameter_remapping;
mod surface_convert;
mod topology;

use std::fs;
use std::path::Path;

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
use crate::step::parameter_remapping::{connect_polylines_with_boundary, remap_2d_points_to_patch};
use crate::step::surface_convert::PatchParamRange;
use crate::surface_trim::QuadraticBezierCurve2d;
use crate::*;

// ── Debug logging ─────────────────────────────────────────────────────

/// Set to `true` to enable verbose step-pipeline tracing via `eprintln!`.
pub const STEP_DEBUG_LOG: bool = false;

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
  stp_file_path: impl AsRef<Path>,
  config: StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  let step_str = fs::read_to_string(stp_file_path.as_ref())
    .map_err(|e| StepReadError::ParseError(format!("Cannot read file: {e}")))?;

  let table =
    Table::from_step(&step_str).ok_or_else(|| StepReadError::ParseError("Parse failed".into()))?;

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
    let patches = match convert_any_surface_to_bezier_patches(&face_data.surface) {
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

    // Process all edges and patches together: tessellate each edge once,
    // project 3D points onto the correct patch (try-last-patch-first),
    // then per-patch connect boundaries + reconstruct.
    let patch_trim_boundaries =
      process_trim_curves_for_face(&patches, &face_data.edges, config, table);

    for (pi, patch) in patches.iter().enumerate() {
      let trim = patch_trim_boundaries
        .get(pi)
        .map(|v| v.to_vec())
        .unwrap_or_default();
      trimmed_surfaces.push(TrimmedSurface {
        surface: patch.surface.clone(),
        trim_boundary: trim,
      });
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

/// Process trim curves for all patches of a face, using single-pass tessellation
/// with distribution of 3D points to the correct patch (try-last-patch heuristic).
///
/// Returns one `Vec<QuadraticBezierCurve2d>` per patch.
fn process_trim_curves_for_face(
  patches: &[PatchParamRange],
  edges: &[EdgeData],
  config: &StepReadConfig,
  table: &Table,
) -> Vec<Vec<QuadraticBezierCurve2d<f32>>> {
  let n_patches = patches.len();
  if n_patches == 0 || edges.is_empty() {
    return std::iter::repeat_with(Vec::new).take(n_patches).collect();
  }

  // per_patch_polylines[pi][ei] = polyline for edge ei on patch pi
  let n_edges = edges.len();
  let mut per_patch_polylines: Vec<Vec<Vec<Vec2<f32>>>> =
    vec![vec![Vec::new(); n_edges]; n_patches];

  let proj_dist_tolerance = config.project_tolerance * 10.0;

  for (ei, edge) in edges.iter().enumerate() {
    // Try pcurve path first — remap to each patch directly
    if try_pcurve_for_all_patches(ei, edge, patches, table, &mut per_patch_polylines) {
      continue;
    }

    // Numerical path: tessellate once, project points onto correct patch
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

    // Project each 3D point onto the correct patch.
    // Use last-successful-patch heuristic with distance check to avoid
    // adding points to the wrong patch (project_point clamps to [0,1]²
    // so wrong-patch projections still "succeed" but with large distance).
    let mut last_patch: Option<usize> = None;
    for &p3 in &all_3d_points {
      // Try last successful patch first
      if let Some(lp) = last_patch {
        if let Some((u, v, d)) = patches[lp].surface.project_point(
          p3,
          config.project_grid,
          config.project_tolerance,
          config.project_max_iter,
        ) {
          if d < proj_dist_tolerance {
            per_patch_polylines[lp][ei].push(Vec2::new(u, v));
            continue;
          }
        }
      }

      // Try all patches
      for (pi, patch) in patches.iter().enumerate() {
        if let Some((u, v, d)) = patch.surface.project_point(
          p3,
          config.project_grid,
          config.project_tolerance,
          config.project_max_iter,
        ) {
          if d < proj_dist_tolerance {
            per_patch_polylines[pi][ei].push(Vec2::new(u, v));
            last_patch = Some(pi);
            break;
          }
        }
      }
    }
  }

  // Phase 2: per-patch boundary connection + reconstruction
  let boundary_epsilon = config.fit_tolerance * 10.0;
  per_patch_polylines
    .into_iter()
    .map(|edge_polys| {
      let polylines: Vec<Vec<Vec2<f32>>> =
        edge_polys.into_iter().filter(|p| !p.is_empty()).collect();

      if polylines.is_empty() {
        return Vec::new();
      }

      let components = connect_polylines_with_boundary(polylines, boundary_epsilon);
      let mut quadratics = Vec::new();
      for component in &components {
        if component.len() >= 2 {
          let qs = crate::surface_trim::reconstruct_boundary(component, config.fit_tolerance);
          quadratics.extend(qs);
        }
      }
      quadratics
    })
    .collect()
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
