use rendiation_algebra::*;
use rendiation_shader_api::*;

pub use crate::bezier_device_shared::BINOMIAL_COEFFICIENTS;
pub use crate::bezier_device_shared::MAX_GPU_DEGREE;
use crate::surface::RationalBezierSurface;

/// Maximum number of control points: (MAX_GPU_DEGREE + 1)².
pub const MAX_GPU_CONTROL_POINTS: usize = (MAX_GPU_DEGREE + 1) * (MAX_GPU_DEGREE + 1);

/// GPU-side Bézier surface metadata.
#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
pub struct GpuBezierSurfaceInfo {
  pub u_degree: u32,
  pub v_degree: u32,
}

impl Default for GpuBezierSurfaceInfo {
  fn default() -> Self {
    Self {
      u_degree: 0,
      v_degree: 0,
      ..unsafe { core::mem::zeroed() }
    }
  }
}

/// GPU-side Bézier control points: homogeneous `Vec4` array.
///
/// Row-major: `index = v_idx * (u_degree + 1) + u_idx`.
/// The array is padded to `MAX_GPU_CONTROL_POINTS` entries; unused entries are zero.
#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
pub struct GpuBezierControlPoints {
  pub data: [Vec4<f32>; MAX_GPU_CONTROL_POINTS],
}

impl Default for GpuBezierControlPoints {
  fn default() -> Self {
    Self {
      data: [Vec4::zero(); MAX_GPU_CONTROL_POINTS],
      ..unsafe { core::mem::zeroed() }
    }
  }
}

impl RationalBezierSurface<f32> {
  /// Export surface metadata for GPU.
  pub fn to_gpu_info(&self) -> GpuBezierSurfaceInfo {
    GpuBezierSurfaceInfo {
      u_degree: self.u_degree() as u32,
      v_degree: self.v_degree() as u32,
      ..Default::default()
    }
  }

  /// Export control points to a padded GPU-compatible array.
  pub fn to_gpu_control_points(&self) -> GpuBezierControlPoints {
    let mut cp: GpuBezierControlPoints = Default::default();
    for (i, p) in self.control_points().iter().enumerate() {
      cp.data[i] = *p;
    }
    cp
  }
}
