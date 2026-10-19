//! Per face trim loop construction in P space. See `DESIGN.md`, section "Face trimming in P
//! space".
//!
//! Loops are first assembled in the surface native uv space (`native`): every edge use becomes
//! an exact native curve (pcurve, or the exact affine projection onto a plane) or a projection
//! of the exact 3D edge with continuation. `assemble` orients the loops (material on the left),
//! unwraps them in periodic axes, inserts pole segments and resolves the shared vertex
//! positions. `domain` picks the native rect of the face, then `to_p` maps every curve into the
//! P space of the rect: exactly (affine map pieces, iso-lines) or fitted with a 3D error bound.

use rendiation_algebra::*;

use crate::brep::*;
use crate::curve::*;
use crate::math::*;
use crate::surface::*;
use crate::*;

mod assemble;
mod domain;
mod iso;
mod native;
#[cfg(test)]
mod tests;
mod to_p;

use assemble::*;
use domain::*;
use native::*;
use to_p::*;

/// A loop in P space: curves connected exactly, material on the left. Closed exactly, or for
/// periodic axes closed up to `winding[axis] * p_period[axis]` (band loop).
#[derive(Clone)]
pub struct PLoop {
  pub curves: Vec<TrimCurve>,
  pub winding: [i32; 2],
}

pub struct FaceTrim {
  pub param_space: SurfaceParamSpace,
  pub loops: Vec<PLoop>,
  /// P space period per axis (from the param maps).
  pub p_period: [Option<f64>; 2],
  /// The loops were reversed because `FACE_SURFACE.same_sense` is false: output patches must
  /// set `flip_normal`.
  pub flip_normal: bool,
  pub fitted_curve_count: usize,
  pub warnings: Vec<String>,
}

pub fn build_face_trim(
  model: &BrepModel,
  face: FaceId,
  surface: &dyn SurfaceModel,
  edges: &mut Edge3Cache,
  ctx: &ConvertCtx,
) -> ConvertResult<FaceTrim> {
  let face = model
    .faces
    .get(face.index())
    .ok_or_else(|| ConvertError::invalid(None, format!("face id {} out of range", face.0)))?;
  let fc = FaceCtx::new(model, face, surface, ctx);
  let mut warnings = Vec::new();

  let mut raw_loops = Vec::new();
  for bound in &face.bounds {
    let BoundKind::Edges(uses) = &bound.kind else {
      continue;
    };
    let uses = bound_uses(uses, bound.orientation);
    let curves =
      build_native_loop(&fc, &uses, edges, &mut warnings).map_err(|e| e.at(bound.step_id))?;
    if !curves.is_empty() {
      raw_loops.push((bound.step_id, curves));
    }
  }

  // In native uv the material is on the left of the bound oriented loops iff `same_sense`.
  let flip_normal = !face.same_sense;
  if flip_normal {
    raw_loops.iter_mut().for_each(|(_, l)| reverse_loop(l));
  }
  let mut loops = assemble_loops(&fc, &raw_loops)?;

  let mut holes_only = false;
  if !loops.is_empty() && loops.iter().all(|l| l.winding == [0, 0]) {
    let area: f64 = loops.iter().map(signed_area).sum();
    if area.is_nan() || area <= 0. {
      if fc.surface_is_closed() {
        holes_only = true;
      } else {
        warnings.push(format!(
          "closed loops enclose a non positive area {area:e} with the material on the left: \
           loop orientation contradicts FACE_SURFACE.same_sense, all loops reversed"
        ));
        raw_loops.iter_mut().for_each(|(_, l)| reverse_loop(l));
        loops = assemble_loops(&fc, &raw_loops)?;
      }
    }
  }

  let rect = face_rect(&fc, &mut loops, holes_only)?;
  let param_space = surface.build_param_space(rect, ctx)?;
  let p_period = [param_space.u_map.p_period(), param_space.v_map.p_period()];
  let mut mapper = LoopMapper::new(&fc, &param_space, p_period);
  let mut p_loops = Vec::with_capacity(loops.len());
  for l in &loops {
    let p_loop = mapper.map_loop(l)?;
    // A loop whose curves all collapsed to points (far below the tolerance) bounds nothing.
    if !p_loop.curves.is_empty() {
      p_loops.push(p_loop);
    }
  }
  let fitted_curve_count = mapper.fitted_curve_count;

  Ok(FaceTrim {
    param_space,
    loops: p_loops,
    p_period,
    flip_normal,
    fitted_curve_count,
    warnings,
  })
}

/// Edge uses in traversal order with the `FACE_BOUND.orientation` folded in.
fn bound_uses(uses: &[OrientedEdgeUse], orientation: bool) -> Vec<OrientedEdgeUse> {
  if orientation {
    return uses.to_vec();
  }
  uses
    .iter()
    .rev()
    .map(|u| OrientedEdgeUse {
      edge: u.edge,
      orientation: !u.orientation,
    })
    .collect()
}

/// Shared per face state.
pub(crate) struct FaceCtx<'a> {
  pub model: &'a BrepModel,
  pub face: &'a Face,
  pub surface: &'a dyn SurfaceModel,
  pub geom: Option<&'a SurfaceGeom>,
  pub ctx: &'a ConvertCtx,
  pub period: [Option<f64>; 2],
  pub poles: Vec<Pole>,
}

impl<'a> FaceCtx<'a> {
  fn new(
    model: &'a BrepModel,
    face: &'a Face,
    surface: &'a dyn SurfaceModel,
    ctx: &'a ConvertCtx,
  ) -> Self {
    Self {
      model,
      face,
      surface,
      geom: model.surfaces.get(face.surface.index()),
      ctx,
      period: surface.period(),
      poles: surface.poles(),
    }
  }

  /// The pole whose position is within `radius` of `p`.
  fn pole_at(&self, p: Vec3<f64>, radius: f64) -> Option<Pole> {
    self
      .poles
      .iter()
      .copied()
      .filter(|pole| (pole.position - p).length() <= radius)
      .min_by(|a, b| {
        let da = (a.position - p).length();
        let db = (b.position - p).length();
        da.total_cmp(&db)
      })
  }

  /// Every axis is periodic or bounded by poles at both ends (sphere, torus, closed B-spline
  /// with degenerate ends): loops enclosing only holes then leave the rest of the surface as
  /// material.
  fn surface_is_closed(&self) -> bool {
    let domain = self.surface.natural_domain();
    (0..2).all(|a| {
      self.period[a].is_some()
        || domain[a].is_some_and(|(lo, hi)| {
          let has = |value: f64| {
            self
              .poles
              .iter()
              .any(|p| p.axis.index() == a && p.value == value)
          };
          has(lo) && has(hi)
        })
    })
  }
}

/// Joint gaps (3D) and projection distances above this many `tol` are data errors.
const GAP_FACTOR: f64 = 10.;

/// `value > limit`, NaN counting as exceeding.
pub(crate) fn exceeds(value: f64, limit: f64) -> bool {
  value.is_nan() || value > limit
}

pub(crate) fn coord(v: Vec2<f64>, axis: usize) -> f64 {
  if axis == 0 { v.x } else { v.y }
}

pub(crate) fn set_coord(v: &mut Vec2<f64>, axis: usize, value: f64) {
  if axis == 0 {
    v.x = value;
  } else {
    v.y = value;
  }
}

pub(crate) fn axis_vec(axis: usize, value: f64) -> Vec2<f64> {
  if axis == 0 {
    Vec2::new(value, 0.)
  } else {
    Vec2::new(0., value)
  }
}
