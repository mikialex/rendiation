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
  /// Placement provenance for each face occurrence, parallel to the
  /// original face occurrence order in the topology walk.
  pub placement_sources: Vec<PlacementSource>,
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
    placement_sources: conversion_result.placement_sources,
  }
}

pub fn read_parametric_rendering_data_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> StepConversionResult {
  assemble_from_table(table, config)
}

fn assemble_from_table(table: &Table, config: &StepReadConfig) -> StepConversionResult {
  use std::collections::HashMap;

  let face_data_list = collect_face_surface_data(table);

  step_dbg!("step: {} faces collected", face_data_list.len());

  let mut trimmed_surfaces: Vec<TrimmedSurface> = Vec::new();
  let mut curves_3d: Vec<RationalBezierCurve3d<f32>> = Vec::new();
  let mut surfaces_instance: Vec<(usize, Mat4<f32>)> = Vec::new();
  let mut curves_3d_instance: Vec<(usize, Mat4<f32>)> = Vec::new();
  let mut placement_sources: Vec<PlacementSource> = Vec::new();
  let mut errors: Vec<StepReadError> = Vec::new();

  // dedup key = (face_id, is_back_face)
  // value = (surface_start_idx, surface_count, curve_start_idx, curve_count)
  let mut face_dedup: HashMap<(u64, bool), (usize, usize, usize, usize)> = HashMap::new();

  for (fi, face_data) in face_data_list.iter().enumerate() {
    let instance_matrix = face_data
      .placement
      .as_ref()
      .map(|p| placement_to_mat4(p))
      .unwrap_or_else(Mat4::identity);

    placement_sources.push(face_data.placement_source.clone());

    let dedup_key = (face_data.face_id, face_data.is_back_face);

    if let Some(&(surf_start, surf_count, curve_start, curve_count)) = face_dedup.get(&dedup_key) {
      // Geometry already exists — only push instance entries.
      for pi in 0..surf_count {
        surfaces_instance.push((surf_start + pi, instance_matrix));
      }
      for ci in 0..curve_count {
        curves_3d_instance.push((curve_start + ci, instance_matrix));
      }
      continue;
    }

    // Pre-convert all edge curves to Bezier once per face.
    let edge_loops_beziers: Vec<Vec<RationalBezierCurve3d<f32>>> = face_data
      .edge_loops
      .iter()
      .flat_map(|l| l.iter())
      .map(|e| match convert_any_curve_to_bezier(&e.curve_3d) {
        Ok(b) => b,
        Err(e) => {
          errors.push(e);
          Vec::new()
        }
      })
      .collect();

    let (patches, original_surface) = match convert_and_split_any_surface_to_bezier_patches(
      &face_data.surface,
      &edge_loops_beziers,
      config,
    ) {
      Ok(p) => p,
      Err(e) => {
        step_dbg!("step: face #{fi} surface conversion failed: {e}");
        errors.push(e);
        // Still record dedup entry so subsequent identical faces get skipped.
        face_dedup.insert(dedup_key, (0, 0, 0, 0));
        continue;
      }
    };

    let sub_patch_count = patches.len();
    step_dbg!("step: face #{fi} → {} patches", patches.len(),);

    let trimmed_patches = process_trim_curves_for_face(
      &original_surface,
      patches,
      &face_data.edge_loops,
      config,
      table,
      &edge_loops_beziers,
    );

    if face_data.placement.is_some() {
      step_dbg!("step: face #{fi} has placement (stored as instance, not baked)");
    }

    let surf_start_idx = trimmed_surfaces.len();
    for tp in trimmed_patches {
      let total_edges: usize = face_data.edge_loops.iter().map(|l| l.len()).sum();
      let debug_label = format!(
        "FaceSurface#{}[{}/{}] {} edges={}{}",
        face_data.face_id,
        tp.patch_index,
        sub_patch_count,
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
        surface: tp.patch.surface,
        trim_loops: tp.trim_loops,
        is_back_face: face_data.is_back_face,
      });
    }
    let surf_count = trimmed_surfaces.len() - surf_start_idx;
    for offset in 0..surf_count {
      surfaces_instance.push((surf_start_idx + offset, instance_matrix));
    }

    let curve_start_idx = curves_3d.len();
    for beziers in &edge_loops_beziers {
      if beziers.is_empty() {
        continue;
      }
      let bez_count = beziers.len();
      let curve_inst_start = curves_3d.len();
      curves_3d.extend(beziers.clone());
      for ci in 0..bez_count {
        curves_3d_instance.push((curve_inst_start + ci, instance_matrix));
      }
    }
    let curve_count = curves_3d.len() - curve_start_idx;

    face_dedup.insert(
      dedup_key,
      (surf_start_idx, surf_count, curve_start_idx, curve_count),
    );
  }

  step_dbg!(
    "step: done — {} trimmed surfaces, {} 3d curves, {} surface instances, {} curve instances",
    trimmed_surfaces.len(),
    curves_3d.len(),
    surfaces_instance.len(),
    curves_3d_instance.len()
  );
  StepConversionResult {
    data: ParametricRenderingData {
      surfaces: trimmed_surfaces,
      curves_3d,
      surfaces_instance,
      curves_3d_instance,
    },
    parse_errors: rendiation_step_reader::table::TableParseErrors::empty(),
    conversion_errors: errors,
    placement_sources,
  }
}
