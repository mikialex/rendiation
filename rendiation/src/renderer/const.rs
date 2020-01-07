use rendiation_math::*;

#[cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: Mat4<f32> = Mat4::const_new(
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
