use curve3d::RationalBezierCurve3d;
use surface::RationalBezierSurface;
use surface_trim::QuadraticBezierCurve2d;

pub mod bezier_curve3d_device;
pub mod bezier_device_shared;
pub mod bezier_surface_device;
pub mod curve3d;
pub mod step;
pub mod surface;
pub mod surface_trim;

pub struct ParametricRenderingData {
  pub surfaces: Vec<TrimmedSurface>,
  pub curves_3d: Vec<RationalBezierCurve3d<f32>>,
}

pub struct TrimmedSurface {
  pub surface: RationalBezierSurface<f32>,
  /// if empty, the surface is not trimmed
  pub trim_boundary: Vec<QuadraticBezierCurve2d<f32>>,
}

use rendiation_step_reader;
