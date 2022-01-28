use rendiation_algebra::*;

#[derive(Clone, Copy)]
pub enum PrimitiveShaderValueType {
  Uint32,
  Float32,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
  Mat2Float32,
  Mat3Float32,
  Mat4Float32,
}

#[derive(Clone, Copy)]
pub enum PrimitiveShaderValue {
  Uint32(u32),
  Float32(f32),
  Vec2Float32(Vec2<f32>),
  Vec3Float32(Vec3<f32>),
  Vec4Float32(Vec4<f32>),
  Mat2Float32(Mat2<f32>),
  Mat3Float32(Mat3<f32>),
  Mat4Float32(Mat4<f32>),
}

impl From<PrimitiveShaderValue> for PrimitiveShaderValueType {
  fn from(v: PrimitiveShaderValue) -> Self {
    match v {
      PrimitiveShaderValue::Uint32(_) => PrimitiveShaderValueType::Uint32,
      PrimitiveShaderValue::Float32(_) => PrimitiveShaderValueType::Float32,
      PrimitiveShaderValue::Vec2Float32(_) => PrimitiveShaderValueType::Vec2Float32,
      PrimitiveShaderValue::Vec3Float32(_) => PrimitiveShaderValueType::Vec3Float32,
      PrimitiveShaderValue::Vec4Float32(_) => PrimitiveShaderValueType::Vec4Float32,
      PrimitiveShaderValue::Mat2Float32(_) => PrimitiveShaderValueType::Mat2Float32,
      PrimitiveShaderValue::Mat3Float32(_) => PrimitiveShaderValueType::Mat3Float32,
      PrimitiveShaderValue::Mat4Float32(_) => PrimitiveShaderValueType::Mat4Float32,
    }
  }
}
