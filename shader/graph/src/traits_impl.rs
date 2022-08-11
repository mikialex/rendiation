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
impl PrimitiveShaderGraphNodeType for i32 {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Int32;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Int32(*self)
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
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTextureCube {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTexture1D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D1,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTexture3D {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D3,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTexture2DArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderTextureCubeArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Float { filterable: true },
  };
}
impl ShaderGraphNodeType for ShaderDepthTextureCube {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Depth,
  };
}
impl ShaderGraphNodeType for ShaderDepthTexture2DArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Depth,
  };
}
impl ShaderGraphNodeType for ShaderDepthTextureCubeArray {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Depth,
  };
}

/// https://www.w3.org/TR/WGSL/#texturesample
pub trait SingleSampleTarget {
  type Input;
  type Sampler;
  type Output: PrimitiveShaderGraphNodeType;
}

impl SingleSampleTarget for ShaderTexture1D {
  type Input = f32;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderTexture2D {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderDepthTexture2D {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = f32;
}

impl SingleSampleTarget for ShaderTexture3D {
  type Input = Vec3<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderTextureCube {
  type Input = Vec3<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl SingleSampleTarget for ShaderDepthTextureCube {
  type Input = Vec3<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

pub trait ArraySampleTarget {
  type Input;
  type Sampler;
  type Output: PrimitiveShaderGraphNodeType;
}

impl ArraySampleTarget for ShaderTexture2DArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl ArraySampleTarget for ShaderTextureCubeArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = Vec4<f32>;
}

impl ArraySampleTarget for ShaderDepthTexture2DArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = f32;
}

impl ArraySampleTarget for ShaderDepthTextureCubeArray {
  type Input = Vec2<f32>;
  type Sampler = ShaderSampler;
  type Output = f32;
}

impl<T: SingleSampleTarget> Node<T> {
  pub fn sample(&self, sampler: Node<T::Sampler>, position: Node<T::Input>) -> Node<T::Output> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: None,
    }
    .insert_graph()
  }
}

pub trait ShaderArrayTextureSampleIndexType: ShaderGraphNodeType {}
impl ShaderArrayTextureSampleIndexType for u32 {}
impl ShaderArrayTextureSampleIndexType for i32 {}

impl<T: ArraySampleTarget> Node<T> {
  pub fn sample_index(
    &self,
    sampler: Node<T::Sampler>,
    position: Node<T::Input>,
    index: Node<impl ShaderArrayTextureSampleIndexType>,
  ) -> Node<T::Output> {
    ShaderGraphNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: index.handle().into(),
    }
    .insert_graph()
  }
}

impl ShaderGraphNodeType for ShaderSamplerCombinedTexture {
  const TYPE: ShaderValueType = ShaderValueType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Float { filterable: true },
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
