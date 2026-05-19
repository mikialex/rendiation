pub mod bezier_curve_tessellate;
mod recontruct_boundary;

use bezier_curve_tessellate::adaptive_tessellate_bezier_curve;
pub use recontruct_boundary::reconstruct_boundary;
use rendiation_algebra::*;

use crate::{curve3d::RationalBezierCurve3d, surface::RationalBezierSurface};

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

/// Project 3D trim curves onto a surface's parametric domain and reconstruct
/// them as 2D quadratic Bézier curves.
///
/// The pipeline has three stages:
///
/// 1. **Adaptive tessellation** — each 3D trim curve is discretized into a
///    polyline within `tessellate_tolerance` via de Casteljau subdivision.
///
/// 2. **Point projection** — each 3D polyline point is projected onto the
///    surface using grid-search + Gauss-Newton iteration. Points that fail
///    to converge are silently dropped; this naturally handles trim curves
///    that extend beyond the surface boundary.
///
/// 3. **Boundary reconstruction** — the resulting 2D (u,v) points are
///    fitted with quadratic Bézier curves via greedy least-squares,
///    producing a minimal curve sequence within `fit_tolerance`.
///
/// The input `trim_curves` must form a closed loop in 3D. Small gaps
/// between consecutive curves are bridged naturally by the reconstruction
/// step, so exact endpoint matching is not required.
pub fn generate_2d_trim_curve<T: Scalar>(
  surface: &RationalBezierSurface<T>,
  trim_curves: Vec<RationalBezierCurve3d<T>>,
  tessellate_tolerance: T,
  project_grid: usize,
  project_tolerance: T,
  project_max_iter: usize,
  fit_tolerance: T,
) -> Vec<QuadraticBezierCurve2d<T>> {
  let mut uv_points: Vec<Vec2<T>> = Vec::new();

  for curve in trim_curves {
    let pts_3d = adaptive_tessellate_bezier_curve(curve, tessellate_tolerance);
    for p in pts_3d {
      if let Some((u, v, _dist)) =
        surface.project_point(p, project_grid, project_tolerance, project_max_iter)
      {
        uv_points.push(Vec2::new(u, v));
      }
    }
  }

  reconstruct_boundary(&uv_points, fit_tolerance)
}
