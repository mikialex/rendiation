// https://github.com/glium/glium/blob/master/src/uniforms/value.rs

#[derive(Copy, Clone)]
pub struct UniformTypeId(u32);

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
  Mat2([[f32; 2]; 2]),
  /// 3x3 column-major matrix.
  Mat3([[f32; 3]; 3]),
  /// 4x4 column-major matrix.
  Mat4([[f32; 4]; 4]),

  Float(f32),
  Vec2([f32; 2]),
  Vec3([f32; 3]),
  Vec4([f32; 4]),

  Int(i32),
  IntVec2([i32; 2]),
  IntVec3([i32; 3]),
  IntVec4([i32; 4]),

  Bool(bool),
  BoolVec2([bool; 2]),
  BoolVec3([bool; 3]),
  BoolVec4([bool; 4]),
  // todo sampler
}
