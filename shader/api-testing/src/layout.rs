use rendiation_shader_api::*;

use crate::harness::*;

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Default)]
pub struct Std430Matrices {
  pub a: f32,
  pub m3x2: Mat3x2<f32>,
  pub b: f32,
  pub m4x2: Mat4x2<f32>,
  pub c: f32,
  pub m2x4: Mat2x4<f32>,
  pub m3x4: Mat3x4<f32>,
}

#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy, Default)]
pub struct Std140Matrices {
  pub a: f32,
  pub m2x4: Mat2x4<f32>,
  pub b: f32,
  pub m3x4: Mat3x4<f32>,
}

/// the host shareable non square matrices, the host layout must match the WGSL layout
#[test]
fn non_square_matrix_host_layout() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;

    let s = zeroed_val::<Std430Matrices>().expand();
    let s = ENode::<Std430Matrices> { a: f, ..s }.construct().expand();
    keep(s.m3x2 * s.b);
    keep(s.m4x2 * s.c);
    keep(s.m2x4);
    keep(s.m3x4);

    let s = zeroed_val::<Std140Matrices>().expand();
    let s = ENode::<Std140Matrices> { a: f, ..s }.construct().expand();
    keep(s.m2x4 * s.b);
    keep(s.m3x4);
  });
}
