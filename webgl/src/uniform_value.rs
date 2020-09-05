// https://github.com/glium/glium/blob/master/src/uniforms/value.rs

use crate::WebGLProgram;
use rendiation_math::*;
use rendiation_ral::*;
use web_sys::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UniformType {
  Float,
  FloatVec2,
  FloatVec3,
  FloatVec4,
  Int,
  IntVec2,
  IntVec3,
  IntVec4,
  Bool,
  BoolVec2,
  BoolVec3,
  BoolVec4,
  FloatMat2,
  FloatMat3,
  FloatMat4,
  Sampler2d,
  SamplerCube,
}

#[derive(Copy, Clone)]
pub enum UniformValue {
  /// 2x2 column-major matrix.
  Mat2(Mat2<f32>),
  /// 3x3 column-major matrix.
  Mat3(Mat3<f32>),
  /// 4x4 column-major matrix.
  Mat4(Mat4<f32>),

  Float(f32),
  Vec2(Vec2<f32>),
  Vec3(Vec3<f32>),
  Vec4(Vec4<f32>),

  Int(i32),
  IntVec2(Vec2<i32>),
  IntVec3(Vec3<i32>),
  IntVec4(Vec4<i32>),

  Bool(bool),
  BoolVec2(Vec2<bool>),
  BoolVec3(Vec3<bool>),
  BoolVec4(Vec4<bool>),
}

impl WebGLProgram {
  pub fn upload_uniform_value(
    &self,
    value: &UniformValue,
    uniform: UniformTypeId,
    gl: &WebGl2RenderingContext,
  ) {
    let location = Some(self.query_uniform_location(uniform));
    use UniformValue::*;
    match value {
      Float(v) => gl.uniform1fv_with_f32_array(location, &[*v; 1]),
      Vec2(v) => gl.uniform1fv_with_f32_array(location, AsRef::<[f32; 2]>::as_ref(v)),
      Vec3(v) => gl.uniform1fv_with_f32_array(location, AsRef::<[f32; 3]>::as_ref(v)),
      Vec4(v) => gl.uniform1fv_with_f32_array(location, AsRef::<[f32; 4]>::as_ref(v)),

      Int(v) => gl.uniform1iv_with_i32_array(location, &[*v; 1]),
      IntVec2(v) => gl.uniform1iv_with_i32_array(location, AsRef::<[i32; 2]>::as_ref(v)),
      IntVec3(v) => gl.uniform1iv_with_i32_array(location, AsRef::<[i32; 3]>::as_ref(v)),
      IntVec4(v) => gl.uniform1iv_with_i32_array(location, AsRef::<[i32; 4]>::as_ref(v)),

      // Bool(v) => gl.uniform1iv_with_i32_array(location, &[*v; 1]),
      // BoolVec2(v) => gl.uniform1iv_with_i32_array(location, AsRef::<[i32; 2]>::as_ref(v)),
      // BoolVec3(v) => gl.uniform1iv_with_i32_array(location, AsRef::<[i32; 3]>::as_ref(v)),
      // BoolVec4(v) => gl.uniform1iv_with_i32_array(location, AsRef::<[i32; 4]>::as_ref(v)),
      Mat2(v) => gl.uniform1fv_with_f32_array(location, AsRef::<[f32; 4]>::as_ref(v)),
      Mat3(v) => gl.uniform1fv_with_f32_array(location, AsRef::<[f32; 9]>::as_ref(v)),
      Mat4(v) => gl.uniform1fv_with_f32_array(location, AsRef::<[f32; 16]>::as_ref(v)),
      _ => {}
    }
  }
}
