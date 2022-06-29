use crate::*;

impl<T: PrimitiveShaderGraphNodeType> ShaderGraphNodeType for T {
  const TYPE: ShaderValueType =
    ShaderValueType::Fixed(ShaderStructMemberValueType::Primitive(T::PRIMITIVE_TYPE));
}

impl<T: PrimitiveShaderGraphNodeType> ShaderStructMemberValueNodeType for T {
  const TYPE: ShaderStructMemberValueType =
    ShaderStructMemberValueType::Primitive(T::PRIMITIVE_TYPE);
}

impl ShaderGraphNodeType for AnyType {
  const TYPE: ShaderValueType = ShaderValueType::Never;
}

impl PrimitiveShaderGraphNodeType for bool {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Bool;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Bool(*self)
  }
}

impl PrimitiveShaderGraphNodeType for u32 {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Uint32;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Uint32(*self)
  }
}

impl PrimitiveShaderGraphNodeType for f32 {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Float32;
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
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Vec2Float32;
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
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Vec3Float32;
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
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Vec4Float32;
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
  const TYPE: ShaderValueType = ShaderValueType::Sampler;
}
impl ShaderGraphNodeType for ShaderCompareSampler {
  const TYPE: ShaderValueType = ShaderValueType::CompareSampler;
}

impl PrimitiveShaderGraphNodeType for Mat2<f32> {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Mat2Float32;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Mat2Float32(*self)
  }
}

impl PrimitiveShaderGraphNodeType for Mat3<f32> {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Mat3Float32;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Mat3Float32(*self)
  }
}

impl PrimitiveShaderGraphNodeType for Mat4<f32> {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Mat4Float32;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Mat4Float32(*self)
  }
}

impl ShaderGraphNodeType for ShaderTexture2D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2,
  };
}

impl Node<ShaderTexture2D> {
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
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2,
  };
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

impl<T: ShaderStructMemberValueNodeType, const N: usize> ShaderStructMemberValueNodeType
  for [T; N]
{
  const TYPE: ShaderStructMemberValueType =
    ShaderStructMemberValueType::FixedSizeArray((&T::TYPE, N));
}
