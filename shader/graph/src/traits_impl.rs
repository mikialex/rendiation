use crate::*;

impl<T: PrimitiveShaderGraphNodeType> ShaderGraphNodeType for T {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Fixed(ShaderStructMemberValueType::Primitive(
      T::to_primitive_type(),
    ))
  }
}

impl<T: PrimitiveShaderGraphNodeType> ShaderStructMemberValueNodeType for T {
  fn to_type() -> ShaderStructMemberValueType {
    ShaderStructMemberValueType::Primitive(T::to_primitive_type())
  }
}

impl ShaderGraphNodeType for AnyType {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Never
  }
}

impl PrimitiveShaderGraphNodeType for bool {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Bool
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Bool(*self)
  }
}

impl PrimitiveShaderGraphNodeType for u32 {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Uint32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Uint32(*self)
  }
}

impl PrimitiveShaderGraphNodeType for f32 {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Float32(*self)
  }
}

impl VertexInShaderGraphNodeType for f32 {
  fn to_vertex_format() -> VertexFormat {
    VertexFormat::Float32
  }
}

impl PrimitiveShaderGraphNodeType for Vec2<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Vec2Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Vec2Float32(*self)
  }
}
impl VertexInShaderGraphNodeType for Vec2<f32> {
  fn to_vertex_format() -> VertexFormat {
    VertexFormat::Float32x2
  }
}

impl PrimitiveShaderGraphNodeType for Vec3<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Vec3Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Vec3Float32(*self)
  }
}
impl VertexInShaderGraphNodeType for Vec3<f32> {
  fn to_vertex_format() -> VertexFormat {
    VertexFormat::Float32x3
  }
}

impl PrimitiveShaderGraphNodeType for Vec4<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Vec4Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Vec4Float32(*self)
  }
}
impl VertexInShaderGraphNodeType for Vec4<f32> {
  fn to_vertex_format() -> VertexFormat {
    VertexFormat::Float32x4
  }
}

impl ShaderGraphNodeType for ShaderSampler {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Sampler
  }
}

impl PrimitiveShaderGraphNodeType for Mat4<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Mat4Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Mat4Float32(*self)
  }
}

impl ShaderGraphNodeType for ShaderTexture {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Texture
  }
}

impl Node<ShaderTexture> {
  pub fn sample(&self, sampler: Node<ShaderSampler>, position: Node<Vec2<f32>>) -> Node<Vec4<f32>> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
    }
    .insert_graph()
  }
}

impl ShaderGraphNodeType for ShaderSamplerCombinedTexture {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Texture
  }
}

impl Node<ShaderSamplerCombinedTexture> {
  pub fn sample(&self, position: Node<Vec2<f32>>) -> Node<Vec4<f32>> {
    ShaderGraphNodeExpr::SamplerCombinedTextureSampling {
      texture: self.handle(),
      position: position.handle(),
    }
    .insert_graph()
  }
}
