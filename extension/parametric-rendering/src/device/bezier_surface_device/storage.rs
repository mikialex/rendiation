use crate::*;

/// Maximum number of control points: (MAX_GPU_DEGREE + 1)².
pub const MAX_GPU_CONTROL_POINTS: usize = (MAX_GPU_DEGREE + 1) * (MAX_GPU_DEGREE + 1);

/// GPU-side Bézier surface metadata.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub struct GpuBezierSurfaceInfo {
  pub u_degree: u32,
  pub v_degree: u32,
}

/// GPU-side Bézier control points: homogeneous `Vec4` array.
///
/// Row-major: `index = v_idx * (u_degree + 1) + u_idx`.
/// The array is padded to `MAX_GPU_CONTROL_POINTS` entries; unused entries are zero.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy)]
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

/// Convert a surface for GPU upload. Returns `None` if a degree exceeds `MAX_GPU_DEGREE`.
pub fn bezier_surface_to_gpu(
  surface: &RationalBezierSurface,
) -> Option<(GpuBezierSurfaceInfo, GpuBezierControlPoints)> {
  if surface.degree_u > MAX_GPU_DEGREE || surface.degree_v > MAX_GPU_DEGREE {
    return None;
  }
  let info = GpuBezierSurfaceInfo {
    u_degree: surface.degree_u as u32,
    v_degree: surface.degree_v as u32,
    ..Default::default()
  };
  let mut cp = GpuBezierControlPoints::default();
  for (dst, src) in cp.data.iter_mut().zip(&surface.control_points) {
    *dst = src.into_f32();
  }
  Some((info, cp))
}
