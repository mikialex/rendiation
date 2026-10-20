//! STEP surface geometry: native evaluation, projection, and per face P space with exact Bézier
//! patches. See `DESIGN.md`, section "Parameter spaces".

use rendiation_algebra::*;

use crate::brep::*;
use crate::math::*;
use crate::*;

mod axis;
mod bspline;
mod elementary;
mod plane;
mod project;
mod revolve;
mod swept;
#[cfg(test)]
mod tests;

use axis::*;
use bspline::*;
use elementary::*;
use plane::*;
use project::*;
use revolve::*;
use swept::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeRect {
  pub u: (f64, f64),
  pub v: (f64, f64),
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceProjection {
  pub native: Vec2<f64>,
  pub distance: f64,
}

/// A degenerate iso-line: every point with native `axis` coordinate equal to `value` maps to the
/// same 3D point (sphere pole: `axis = V`, `value = ±π/2`; cone apex: `axis = V`).
#[derive(Debug, Clone, Copy)]
pub struct Pole {
  pub axis: Axis,
  pub value: f64,
  pub position: Vec3<f64>,
}

/// The face specific P space: per axis maps, and one exact patch per existing cell.
pub struct SurfaceParamSpace {
  pub u_map: PiecewiseParamMap,
  pub v_map: PiecewiseParamMap,
  /// Cell `(i, j)` covers P `[i, i+1] × [j, j+1]`; `patch.eval(p - (i, j)) == eval(native(p))`.
  pub patches: Vec<CellPatch>,
}

pub struct CellPatch {
  pub cell: (usize, usize),
  pub surface: RationalBezierSurface,
}

impl SurfaceParamSpace {
  pub fn to_p(&self, native: Vec2<f64>) -> Vec2<f64> {
    Vec2::new(self.u_map.to_p(native.x), self.v_map.to_p(native.y))
  }

  pub fn to_native(&self, p: Vec2<f64>) -> Vec2<f64> {
    Vec2::new(self.u_map.to_native(p.x), self.v_map.to_native(p.y))
  }
}

pub trait SurfaceModel {
  fn kind_name(&self) -> &'static str;

  /// Evaluation in native parameters (ISO 10303-42 definitions).
  fn eval(&self, native: Vec2<f64>) -> Vec3<f64>;

  /// Point, ∂S/∂u, ∂S/∂v in native parameters.
  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>);

  /// Native period per axis.
  fn period(&self) -> [Option<f64>; 2];

  /// Natural native bounds per axis, `None` when unbounded (plane, cylinder `v`...). Periodic
  /// axes return `None` here.
  fn natural_domain(&self) -> [Option<(f64, f64)>; 2];

  /// Start of the base period of each periodic axis (`None` for non periodic axes). Before
  /// calling `build_param_space`, shift periodic rect coordinates so the rect starts inside
  /// `[start, start + period)`: surfaces with a fixed periodic map (closed B-splines, swept
  /// closed curves) require it, analytic angular axes accept any start.
  fn period_start(&self) -> [Option<f64>; 2] {
    self.period().map(|p| p.map(|_| 0.))
  }

  fn poles(&self) -> Vec<Pole>;

  /// Closest point on the surface. `guess` (native) enables a fast local search and must be
  /// used for continuation along curves; the result is the closest point near the guess when
  /// several minima exist. Periodic coordinates are returned in the representative closest to
  /// `guess` when given.
  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection>;

  /// Build the P space and the patches needed to cover `rect` (native, already unwrapped for
  /// periodic axes; may exceed one period only if it spans exactly one period).
  ///
  /// Angular axes of analytic surfaces (and of swept conics / the revolution angle) get equal
  /// arcs of at most π/2 starting at the rect start, periodic when the rect spans a full turn.
  /// Affine axes of analytic surfaces get one segment over exactly the rect. Fixed maps
  /// (B-spline knot spans, swept curve segments) cover the whole domain with P = 0 at its
  /// start, only cells intersecting the rect get patches; periodic coordinates must start in
  /// the base period (see `period_start`) and non periodic ones must lie inside
  /// `natural_domain`, otherwise an error is returned.
  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace>;
}

/// Build the native surface model of `surface`. Errors for offset surfaces and unsupported
/// surface types, and for invalid geometry (non positive radii, broken B-spline data...).
///
/// Swept surfaces (extrusion, revolution) resolve their swept curve here through
/// `curve::build_curve_param_space`, the returned model does not borrow `model`.
pub fn build_surface(
  model: &BrepModel,
  surface: SurfaceId,
  ctx: &ConvertCtx,
) -> ConvertResult<Box<dyn SurfaceModel>> {
  let geom = model
    .surfaces
    .get(surface.index())
    .ok_or_else(|| ConvertError::invalid(None, format!("dangling surface id {surface:?}")))?;
  Ok(match geom {
    SurfaceGeom::Plane(frame) => Box::new(Plane::new(*frame)),
    SurfaceGeom::Cylinder { frame, radius } => Box::new(Cylinder::new(*frame, *radius)?),
    SurfaceGeom::Cone {
      frame,
      radius,
      semi_angle,
    } => Box::new(Cone::new(*frame, *radius, *semi_angle)?),
    SurfaceGeom::Sphere { frame, radius } => Box::new(Sphere::new(*frame, *radius)?),
    SurfaceGeom::Torus {
      frame,
      major_radius,
      minor_radius,
    } => Box::new(Torus::new(*frame, *major_radius, *minor_radius)?),
    SurfaceGeom::BSpline(data) => Box::new(BSplineSurface::new(data, ctx)?),
    SurfaceGeom::Extrusion { curve, vector } => Box::new(Extrusion::new(
      SweptCurve::new(model, *curve, ctx)?,
      *vector,
      ctx,
    )?),
    SurfaceGeom::Revolution {
      curve,
      axis_origin,
      axis_dir,
    } => Box::new(Revolution::new(
      SweptCurve::new(model, *curve, ctx)?,
      *axis_origin,
      *axis_dir,
      ctx,
    )?),
    SurfaceGeom::Offset { .. } => {
      return Err(ConvertError::unsupported(None, "offset surface"));
    }
    SurfaceGeom::Unsupported { step_id, type_name } => {
      return Err(ConvertError::unsupported(
        Some(*step_id),
        format!("unsupported surface type {type_name}"),
      ));
    }
  })
}
