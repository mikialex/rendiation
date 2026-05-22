use crate::*;

/// Maximum number of control points for a single curve: (MAX_GPU_DEGREE + 1).
pub const MAX_GPU_CURVE_CONTROL_POINTS: usize = MAX_GPU_DEGREE + 1;

/// GPU-side Bézier curve metadata.
#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
pub struct GpuBezierCurveInfo {
  pub degree: u32,
}

impl Default for GpuBezierCurveInfo {
  fn default() -> Self {
    Self {
      degree: 0,
      ..unsafe { core::mem::zeroed() }
    }
  }
}

/// GPU-side Bézier curve control points: homogeneous `Vec4` array.
///
/// The array is padded to `MAX_GPU_CURVE_CONTROL_POINTS` entries; unused entries are zero.
#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
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

impl RationalBezierCurve3d<f32> {
  /// Export curve metadata for GPU.
  pub fn to_gpu_info(&self) -> GpuBezierCurveInfo {
    GpuBezierCurveInfo {
      degree: self.degree() as u32,
      ..Default::default()
    }
  }

  /// Export control points to a padded GPU-compatible array.
  pub fn to_gpu_control_points(&self) -> GpuBezierCurveControlPoints {
    let mut cp: GpuBezierCurveControlPoints = Default::default();
    for (i, p) in self.control_points().iter().enumerate() {
      cp.data[i] = *p;
    }
    cp
  }
}
