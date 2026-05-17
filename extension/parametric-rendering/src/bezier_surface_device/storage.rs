use rendiation_algebra::*;
use rendiation_shader_api::*;

use crate::surface::RationalBezierSurface;

/// Maximum supported degree for Bernstein evaluation on GPU.
pub const MAX_GPU_DEGREE: usize = 14;

/// Maximum number of control points: (MAX_GPU_DEGREE + 1)².
pub const MAX_GPU_CONTROL_POINTS: usize = (MAX_GPU_DEGREE + 1) * (MAX_GPU_DEGREE + 1);

/// Binomial coefficients C(n,k) for n = 0..=14, k = 0..=n.
///
/// Layout: 15 rows × 16 columns, row-major.
/// Index: `BINOMIAL_COEFFICIENTS[(degree - 1) * 16 + k]` for degree ∈ [1,14], k ∈ [0, degree].
/// Row 0 stores C(1,*); row 13 stores C(14,*).
/// Unused entries are 0.
pub const BINOMIAL_COEFFICIENTS: [f32; 240] = [
  // degree 0
  1., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., // degree 1
  1., 2., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., // degree 2
  1., 3., 3., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., // degree 3
  1., 4., 6., 4., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., // degree 4
  1., 5., 10., 10., 5., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., // degree 5
  1., 6., 15., 20., 15., 6., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., // degree 6
  1., 7., 21., 35., 35., 21., 7., 1., 0., 0., 0., 0., 0., 0., 0., 0., // degree 7
  1., 8., 28., 56., 70., 56., 28., 8., 1., 0., 0., 0., 0., 0., 0., 0., // degree 8
  1., 9., 36., 84., 126., 126., 84., 36., 9., 1., 0., 0., 0., 0., 0., 0., // degree 9
  1., 10., 45., 120., 210., 252., 210., 120., 45., 10., 1., 0., 0., 0., 0., 0.,
  // degree 10
  1., 11., 55., 165., 330., 462., 462., 330., 165., 55., 11., 1., 0., 0., 0., 0.,
  // degree 11
  1., 12., 66., 220., 495., 792., 924., 792., 495., 220., 66., 12., 1., 0., 0., 0.,
  // degree 12
  1., 13., 78., 286., 715., 1287., 1716., 1716., 1287., 715., 286., 78., 13., 1., 0., 0.,
  // degree 13
  1., 14., 91., 364., 1001., 2002., 3003., 3432., 3003., 2002., 1001., 364., 91., 14., 1., 0.,
  // degree 14
  1., 15., 105., 455., 1365., 3003., 5005., 6435., 6435., 5005., 3003., 1365., 455., 105., 15., 1.,
];

/// GPU-side Bézier surface metadata.
///
/// Stored in a uniform or storage buffer for access by the compute shader.
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
