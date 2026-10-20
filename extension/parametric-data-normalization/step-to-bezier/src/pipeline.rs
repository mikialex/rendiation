use std::ops::Range;

use parametric_normalized_model::{EdgeInfo, FaceInfo};
use rendiation_algebra::*;
use step_reader::table::{Table, TableParseErrors};

use crate::brep::*;
use crate::curve::Edge3Cache;
use crate::face::build_face_trim;
use crate::surface::build_surface;
use crate::trim::{CellTrim, TrimParams, trim_cells};
use crate::*;

#[derive(Debug, Clone)]
pub struct ConvertConfig {
  /// Model tolerance override (file length units). Default: STEP uncertainty, otherwise a size
  /// based fallback.
  pub tolerance: Option<f64>,
  /// `fit_tol = tol * fit_tolerance_ratio`.
  pub fit_tolerance_ratio: f64,
  pub fit_degree: usize,
}

impl Default for ConvertConfig {
  fn default() -> Self {
    Self {
      tolerance: None,
      fit_tolerance_ratio: 0.1,
      fit_degree: 3,
    }
  }
}

/// Everything produced by a conversion: the output model, the per face report, and the
/// resolved B-rep (useful for validation and debugging).
pub struct ConvertSession {
  pub model: ParametricNormalizedModel,
  pub report: ConvertReport,
  pub brep: BrepModel,
  pub ctx: ConvertCtx,
}

pub fn convert_step(step: &str, config: &ConvertConfig) -> ConvertSession {
  let parsed = Table::from_step(step, None);
  let mut session = convert_table(&parsed.table, config);
  session.report.parse_errors = match parsed.errors {
    TableParseErrors::StepSyntaxError(msg) => vec![format!("syntax error: {msg}")],
    TableParseErrors::NoDataSection => vec!["no data section".to_string()],
    TableParseErrors::EntityErrors(errors) => errors.iter().map(|e| e.to_string()).collect(),
  };
  session
}

pub fn convert_table(table: &Table, config: &ConvertConfig) -> ConvertSession {
  let model = build_brep_model(table);
  let ctx = ConvertCtx::new(
    resolve_tolerance(&model, config),
    config.fit_tolerance_ratio,
    config.fit_degree,
  );

  let mut edges = Edge3Cache::default();
  let mut patches = Vec::new();
  let mut face_patches: Vec<Option<Range<usize>>> = vec![None; model.faces.len()];
  let mut report = ConvertReport {
    brep_warnings: model.warnings.clone(),
    tol: ctx.tol,
    ..Default::default()
  };

  for face_id in referenced_faces(&model) {
    let face = &model.faces[face_id.index()];
    let surface_kind = surface_kind_name(&model.surfaces[face.surface.index()]);
    let mut warnings = Vec::new();
    let result = convert_face(&model, face_id, &mut edges, &ctx, &mut warnings).map(|converted| {
      let start = patches.len();
      let stats = FaceStats {
        patch_count: converted.patches.len(),
        trim_curve_count: converted
          .patches
          .iter()
          .flat_map(|p| &p.loops)
          .map(|l| l.curves.len())
          .sum(),
        fitted_curve_count: converted.fitted_curve_count,
      };
      patches.extend(converted.patches);
      face_patches[face_id.index()] = Some(start..patches.len());
      stats
    });
    report.faces.push(FaceReport {
      face: face_id,
      step_id: face.step_id,
      surface_kind,
      result,
      warnings,
    });
  }

  let mut instances = Vec::new();
  for occurrence in &model.occurrences {
    let Some(body) = model.bodies.get(occurrence.body.index()) else {
      continue;
    };
    for &(face, orientation) in body.shells.iter().flatten() {
      if let Some(range) = &face_patches[face.index()] {
        instances.push(FaceInstance {
          face,
          patches: range.clone(),
          transform: occurrence.transform,
          flip_normal: !orientation,
        });
      }
    }
  }

  ConvertSession {
    model: ParametricNormalizedModel {
      patches,
      instances,
      faces: model
        .faces
        .iter()
        .map(|f| FaceInfo {
          source_id: f.step_id,
        })
        .collect(),
      edges: model
        .edges
        .iter()
        .map(|e| EdgeInfo {
          source_id: e.step_id,
        })
        .collect(),
      tolerance: ctx.tol,
      length_unit_mm: model.length_unit_mm,
    },
    report,
    brep: model,
    ctx,
  }
}

struct ConvertedFace {
  patches: Vec<TrimmedBezierPatch>,
  fitted_curve_count: usize,
}

fn convert_face(
  model: &BrepModel,
  face_id: FaceId,
  edges: &mut Edge3Cache,
  ctx: &ConvertCtx,
  warnings: &mut Vec<String>,
) -> ConvertResult<ConvertedFace> {
  let face = &model.faces[face_id.index()];
  let surface = build_surface(model, face.surface, ctx).map_err(|e| e.at(face.step_id))?;
  let trim = build_face_trim(model, face_id, surface.as_ref(), edges, ctx)
    .map_err(|e| e.at(face.step_id))?;
  warnings.extend(trim.warnings.iter().cloned());

  let cells: Vec<_> = trim.param_space.patches.iter().map(|p| p.cell).collect();
  let trimmed = trim_cells(&trim.loops, trim.p_period, &cells, &TrimParams::default())
    .map_err(|e| e.at(face.step_id))?;

  let patches = trim
    .param_space
    .patches
    .iter()
    .zip(trimmed)
    .filter_map(|(patch, cell_trim)| match cell_trim {
      CellTrim::Empty => None,
      CellTrim::Trimmed(loops) => Some(TrimmedBezierPatch {
        surface: patch.surface.clone(),
        loops,
        flip_normal: trim.flip_normal,
        face: face_id,
        cell: patch.cell,
      }),
    })
    .collect();

  Ok(ConvertedFace {
    patches,
    fitted_curve_count: trim.fitted_curve_count,
  })
}

/// Faces used by at least one body shell, in id order. Faces not reachable from any body are
/// not converted.
fn referenced_faces(model: &BrepModel) -> Vec<FaceId> {
  let mut used = vec![false; model.faces.len()];
  for body in &model.bodies {
    for &(face, _) in body.shells.iter().flatten() {
      if let Some(u) = used.get_mut(face.index()) {
        *u = true;
      }
    }
  }
  (0..model.faces.len())
    .filter(|i| used[*i])
    .map(|i| FaceId(i as u32))
    .collect()
}

/// Explicit override, else the STEP uncertainty clamped below by a fraction of the model size
/// (some files declare unrealistic uncertainties like 1e-13), else a fraction of the model size.
fn resolve_tolerance(model: &BrepModel, config: &ConvertConfig) -> f64 {
  if let Some(tol) = config.tolerance.filter(|t| t.is_finite() && *t > 0.) {
    return tol;
  }
  let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
  let mut max = min * -1.;
  for v in &model.vertices {
    let p = v.position;
    min = Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
    max = Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
  }
  let diagonal = (max - min).length();
  let size_tol = if diagonal.is_finite() && diagonal > 0. {
    diagonal * 1e-6
  } else {
    1e-6
  };
  match model.uncertainty.filter(|t| t.is_finite() && *t > 0.) {
    Some(tol) => tol.max(size_tol * 0.1),
    None => size_tol,
  }
}

fn surface_kind_name(surface: &SurfaceGeom) -> &'static str {
  match surface {
    SurfaceGeom::Plane(_) => "Plane",
    SurfaceGeom::Cylinder { .. } => "Cylinder",
    SurfaceGeom::Cone { .. } => "Cone",
    SurfaceGeom::Sphere { .. } => "Sphere",
    SurfaceGeom::Torus { .. } => "Torus",
    SurfaceGeom::BSpline(_) => "BSpline",
    SurfaceGeom::Extrusion { .. } => "Extrusion",
    SurfaceGeom::Revolution { .. } => "Revolution",
    SurfaceGeom::Offset { .. } => "Offset",
    SurfaceGeom::Unsupported { .. } => "Unsupported",
  }
}
