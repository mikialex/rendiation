mod curve2d_sample;
mod curve3d_convert;
mod helpers;
mod parameter_remapping;
mod placement;
mod reconstruct_boundary;
mod sub_patch_trim;
mod surface_convert;
mod surface_extent;
mod surface_project;
mod topology;

use curve2d_sample::*;
use curve3d_convert::*;
use helpers::*;
use parameter_remapping::*;
use placement::*;
use reconstruct_boundary::*;
use rendiation_step_reader::entities::SurfaceAny;
use rendiation_step_reader::table::Table;
use sub_patch_trim::*;
use surface_convert::*;
use surface_extent::*;
use surface_project::*;
use topology::*;

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
  MissingEntity { entity_type: &'static str, id: u64 },
  UnsupportedSurface { entity_type: &'static str, id: u64 },
  UnsupportedCurve { entity_type: &'static str, id: u64 },
  ConversionError(String),
}

impl std::fmt::Display for StepReadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
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

/// Result of the STEP-to-parametric conversion pipeline.
///
/// Always contains the output data (which may be partially populated) and
/// errors classified by origin: table-level parse errors vs. conversion
/// errors encountered during the geometry pipeline.
pub struct StepConversionResult {
  pub data: ParametricRenderingData,
  pub parse_errors: rendiation_step_reader::table::TableParseErrors,
  pub conversion_errors: Vec<StepReadError>,
}

impl StepConversionResult {
  /// Print all errors (both parse and conversion) to stdout.
  ///
  /// Reports entity-level parse errors, syntax errors, no-data-section,
  /// and conversion errors from the geometry pipeline.
  pub fn print_errors(&self) {
    match &self.parse_errors {
      rendiation_step_reader::table::TableParseErrors::EntityErrors(errs) if !errs.is_empty() => {
        println!("step: {} entity parse error(s):", errs.len());
        for e in errs {
          println!("  {e}");
        }
      }
      rendiation_step_reader::table::TableParseErrors::StepSyntaxError(msg) => {
        println!("step: syntax error — {msg}");
      }
      rendiation_step_reader::table::TableParseErrors::NoDataSection => {
        println!("step: no data section");
      }
      _ => {}
    }

    if !self.conversion_errors.is_empty() {
      println!(
        "step: {} conversion error(s):",
        self.conversion_errors.len()
      );
      for e in &self.conversion_errors {
        println!("  {e}");
      }
    }
  }
}

// ── Main entry ────────────────────────────────────────────────────────

pub fn read_parametric_rendering_data_from_step(
  step_str: &str,
  config: StepReadConfig,
) -> StepConversionResult {
  let parse_result = Table::from_step(step_str, None);

  let parse_errors = parse_result.errors;

  match &parse_errors {
    rendiation_step_reader::table::TableParseErrors::EntityErrors(entity_errors)
      if !entity_errors.is_empty() =>
    {
      step_dbg!("step: {} entity parse error(s):", entity_errors.len());
      for e in entity_errors {
        step_dbg!("  {e}");
      }
    }
    rendiation_step_reader::table::TableParseErrors::StepSyntaxError(msg) => {
      step_dbg!("step: syntax error — {msg}");
    }
    rendiation_step_reader::table::TableParseErrors::NoDataSection => {
      step_dbg!("step: no data section");
    }
    _ => {}
  }

  let table = parse_result.table;

  let conversion_result = assemble_from_table(&table, &config);

  StepConversionResult {
    data: conversion_result.data,
    parse_errors,
    conversion_errors: conversion_result.conversion_errors,
  }
}

pub fn read_parametric_rendering_data_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> StepConversionResult {
  assemble_from_table(table, config)
}

fn assemble_from_table(table: &Table, config: &StepReadConfig) -> StepConversionResult {
  let face_data_list = collect_face_surface_data(table);

  step_dbg!("step: {} faces collected", face_data_list.len());

  let mut trimmed_surfaces: Vec<TrimmedSurface> = Vec::new();
  let mut curves_3d: Vec<RationalBezierCurve3d<f32>> = Vec::new();
  let mut errors: Vec<StepReadError> = Vec::new();

  for (fi, face_data) in face_data_list.iter().enumerate() {
    let plane_extent = if let SurfaceAny::Plane(ref p) = &face_data.surface {
      Some(compute_plane_face_extent(
        &p.position,
        &face_data.edge_loops,
        config,
      ))
    } else {
      None
    };

    // todo, check u side?
    let v_range = match &face_data.surface {
      SurfaceAny::CylindricalSurface(c) => Some(compute_axis_v_extent(
        &c.position,
        &face_data.edge_loops,
        config,
      )),
      SurfaceAny::ConicalSurface(c) => Some(compute_axis_v_extent(
        &c.position,
        &face_data.edge_loops,
        config,
      )),
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
        errors.push(e);
        continue;
      }
    };

    step_dbg!(
      "step: face #{fi} → {} patches, {} edges",
      patches.len(),
      face_data.edge_loops.iter().map(|l| l.len()).sum::<usize>()
    );

    let patch_trim_boundaries = process_trim_curves_for_face(
      &original_surface,
      &patches,
      &face_data.edge_loops,
      config,
      table,
    );

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

    let n_patches = patches.len();
    for (pi, patch) in patches.iter().enumerate() {
      match &patch_trim_boundaries[pi] {
        Some(trim) => {
          // fi (face occurrence index) ensures uniqueness when the same
          // face entity appears in multiple assembly placements.
          let total_edges: usize = face_data.edge_loops.iter().map(|l| l.len()).sum();
          let debug_label = format!(
            "#{fi} FaceSurface#{}[{}/{}] {} edges={}{}",
            face_data.face_id,
            pi,
            n_patches,
            face_data.surface.surface_ty_name(),
            total_edges,
            if face_data.is_back_face {
              " (flipped)"
            } else {
              ""
            }
          );
          trimmed_surfaces.push(TrimmedSurface {
            debug_label,
            surface: patch.surface.clone(),
            trim_loops: trim.clone(),
            is_back_face: face_data.is_back_face,
          });
        }
        None => {
          step_dbg!("step: face #{fi} patch #{pi} discarded (fully trimmed)");
        }
      }
    }

    for edge in face_data.edge_loops.iter().flat_map(|l| l.iter()) {
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
          errors.push(e);
        }
      }
    }
  }

  step_dbg!(
    "step: done — {} trimmed surfaces, {} 3d curves",
    trimmed_surfaces.len(),
    curves_3d.len()
  );
  StepConversionResult {
    data: ParametricRenderingData {
      surfaces: trimmed_surfaces,
      curves_3d,
    },
    parse_errors: rendiation_step_reader::table::TableParseErrors::empty(),
    conversion_errors: errors,
  }
}
