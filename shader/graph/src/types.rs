pub enum PrimitiveShaderValueType {
  Float32,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
  Mat2Float32,
  Mat3Float32,
  Mat4Float32,
}

impl PrimitiveShaderValueType {
  pub fn to_glsl(&self) -> &'static str {
    todo!()
  }
}
