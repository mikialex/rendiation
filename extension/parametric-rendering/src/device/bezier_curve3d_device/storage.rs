use crate::*;

/// Maximum number of control points for a single curve: (MAX_GPU_DEGREE + 1).
pub const MAX_GPU_CURVE_CONTROL_POINTS: usize = MAX_GPU_DEGREE + 1;

/// GPU-side Bézier curve metadata.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub struct GpuBezierCurveInfo {
  pub degree: u32,
}

/// GPU-side Bézier curve control points: homogeneous `Vec4` array.
///
/// The array is padded to `MAX_GPU_CURVE_CONTROL_POINTS` entries; unused entries are zero.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy)]
pub struct GpuBezierCurveControlPoints {
  pub data: [Vec4<f32>; MAX_GPU_CURVE_CONTROL_POINTS],
}

impl Default for GpuBezierCurveControlPoints {
  fn default() -> Self {
    Self {
      data: [Vec4::zero(); MAX_GPU_CURVE_CONTROL_POINTS],
      ..unsafe { core::mem::zeroed() }
    }
  }
}

/// Convert a curve for GPU upload. Returns `None` if the degree exceeds `MAX_GPU_DEGREE`.
pub fn bezier_curve_to_gpu(
  curve: &RationalBezierCurve3,
) -> Option<(GpuBezierCurveInfo, GpuBezierCurveControlPoints)> {
  if curve.control_points.is_empty() || curve.degree() > MAX_GPU_DEGREE {
    return None;
  }
  let info = GpuBezierCurveInfo {
    degree: curve.degree() as u32,
    ..Default::default()
  };
  let mut cp = GpuBezierCurveControlPoints::default();
  for (dst, src) in cp.data.iter_mut().zip(&curve.control_points) {
    *dst = src.into_f32();
  }
  Some((info, cp))
}
