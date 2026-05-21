use curve3d::RationalBezierCurve3d;
use surface::RationalBezierSurface;

pub mod bezier_curve3d_device;
pub mod bezier_device_shared;
pub mod bezier_surface_device;
pub mod curve3d;
pub mod mesh;
pub mod step;
pub mod surface;
mod surface_trim;

use curve3d::*;
use rendiation_algebra::*;
use rendiation_step_reader;
use surface::*;
use surface_trim::*;

pub struct ParametricRenderingData {
  pub surfaces: Vec<TrimmedSurface>,
  pub curves_3d: Vec<RationalBezierCurve3d<f32>>,
}

pub struct TrimmedSurface {
  pub debug_label: String,
  pub surface: RationalBezierSurface<f32>,
  /// Trim boundary loops, one per FaceBound.  Each inner vec is a closed
  /// loop of quadratic Bézier curves in the patch-local [0,1]² domain.
  /// Empty vec means the surface is not trimmed.
  pub trim_loops: Vec<Vec<QuadraticBezierCurve2d<f32>>>,
  /// Whether to flip normals (and triangle winding) relative to du×dv.
  /// Propagated from FaceSurface.same_sense / OrientedFace.orientation.
  pub is_back_face: bool,
}

impl TrimmedSurface {
  /// Whether this surface has any trim loops.
  pub fn is_trimmed(&self) -> bool {
    self.trim_loops.iter().any(|l| !l.is_empty())
  }

  /// Flatten all trim loops into a single list (for consumers that don't
  /// care about loop boundaries).
  pub fn all_trim_curves(&self) -> impl Iterator<Item = &QuadraticBezierCurve2d<f32>> {
    self.trim_loops.iter().flat_map(|l| l.iter())
  }
}

/// A quadratic Bézier curve in 2D parametric space.
///
/// Defined by start point, control point, and end point:
/// `B(t) = (1-t)²·start + 2(1-t)t·ctrl + t²·end`
#[derive(Clone)]
pub struct QuadraticBezierCurve2d<T> {
  pub start: Vec2<T>,
  pub ctrl: Vec2<T>,
  pub end: Vec2<T>,
}
