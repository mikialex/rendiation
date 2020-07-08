// https://github.com/glium/glium/blob/master/src/uniforms/value.rs

use crate::{UniformTypeId, WebGLProgram, WebGLRenderer};
use rendiation_math::*;

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
  IntVec2([i32; 2]),
  IntVec3([i32; 3]),
  IntVec4([i32; 4]),

  Bool(bool),
  BoolVec2([bool; 2]),
  BoolVec3([bool; 3]),
  BoolVec4([bool; 4]),
}

impl WebGLProgram {
  pub fn upload_uniform_value(
    &self,
    value: &UniformValue,
    uniform: UniformTypeId,
    renderer: &WebGLRenderer,
  ) {
    let location = self.query_uniform_location(uniform);
    let gl = &renderer.gl;
    use UniformValue::*;
    match value {
      Float(v) => gl.uniform1fv_with_f32_array(Some(location), &[*v; 1]),
      Vec2(v) => {
        let slice: &[f32; 2] = v.as_ref();
        gl.uniform1fv_with_f32_array(Some(location), slice)
      }
      Vec3(v) => {
        let slice: &[f32; 3] = v.as_ref();
        gl.uniform1fv_with_f32_array(Some(location), slice)
      }
      Vec4(v) => {
        let slice: &[f32; 4] = v.as_ref();
        gl.uniform1fv_with_f32_array(Some(location), slice)
      }
      // Mat2(v) => {
      //   let slice: &[f32; 4] = v.as_ref();
      //   gl.uniform1fv_with_f32_array(Some(location), slice)
      // }
      _ => {}
    }
  }
}
