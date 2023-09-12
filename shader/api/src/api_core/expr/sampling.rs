use crate::*;

#[derive(Clone, Copy)]
pub struct ShaderTexture1D;
#[derive(Clone, Copy)]
pub struct ShaderTexture2D;
#[derive(Clone, Copy)]
pub struct ShaderTexture3D;
#[derive(Clone, Copy)]
pub struct ShaderTextureCube;
#[derive(Clone, Copy)]
pub struct ShaderTexture2DArray;
#[derive(Clone, Copy)]
pub struct ShaderTextureCubeArray;
#[derive(Clone, Copy)]
pub struct ShaderDepthTexture2D;
#[derive(Clone, Copy)]
pub struct ShaderDepthTextureCube;
#[derive(Clone, Copy)]
pub struct ShaderDepthTexture2DArray;
#[derive(Clone, Copy)]
pub struct ShaderDepthTextureCubeArray;

#[derive(Clone, Copy)]
pub struct ShaderSampler;
#[derive(Clone, Copy)]
pub struct ShaderCompareSampler;

sg_node_impl!(
  ShaderSampler,
  ShaderValueSingleType::Sampler(SamplerBindingType::Filtering)
);
sg_node_impl!(
  ShaderCompareSampler,
  ShaderValueSingleType::Sampler(SamplerBindingType::Comparison)
);

sg_node_impl!(
  ShaderTexture2D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Float { filterable: true },
  }
);
sg_node_impl!(
  ShaderTextureCube,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Float { filterable: true },
  }
);
sg_node_impl!(
  ShaderTexture1D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D1,
    sample_type: TextureSampleType::Float { filterable: true },
  }
);
sg_node_impl!(
  ShaderTexture3D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D3,
    sample_type: TextureSampleType::Float { filterable: true },
  }
);
sg_node_impl!(
  ShaderTexture2DArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Float { filterable: true },
  }
);
sg_node_impl!(
  ShaderTextureCubeArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Float { filterable: true },
  }
);
sg_node_impl!(
  ShaderDepthTexture2D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Depth,
  }
);
sg_node_impl!(
  ShaderDepthTexture2DArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Depth,
  }
);
sg_node_impl!(
  ShaderDepthTextureCube,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Depth,
  }
);
sg_node_impl!(
  ShaderDepthTextureCubeArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Depth,
  }
);

/// https://www.w3.org/TR/WGSL/#texturesample
pub trait SingleSampleTarget {
  type Input;
  type Sampler;
  type Output: PrimitiveShaderNodeType;
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
  type Output: PrimitiveShaderNodeType;
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

impl<T: SingleSampleTarget> HandleNode<T> {
  pub fn sample(
    &self,
    sampler: HandleNode<T::Sampler>,
    position: impl Into<Node<T::Input>>,
  ) -> Node<T::Output> {
    ShaderNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.into().handle(),
      index: None,
      level: SampleLevel::Auto,
      reference: None,
      offset: None,
    }
    .insert_api()
  }

  pub fn sample_level(
    &self,
    sampler: HandleNode<T::Sampler>,
    position: impl Into<Node<T::Input>>,
    level: impl Into<Node<f32>>,
  ) -> Node<T::Output> {
    ShaderNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.into().handle(),
      index: None,
      level: SampleLevel::Exact(level.into().handle()),
      reference: None,
      offset: None,
    }
    .insert_api()
  }
}

pub trait ShaderArrayTextureSampleIndexType: ShaderNodeType {}
impl ShaderArrayTextureSampleIndexType for u32 {}
impl ShaderArrayTextureSampleIndexType for i32 {}

impl<T: ArraySampleTarget> HandleNode<T> {
  pub fn sample_index(
    &self,
    sampler: HandleNode<T::Sampler>,
    position: Node<T::Input>,
    index: Node<impl ShaderArrayTextureSampleIndexType>,
  ) -> Node<T::Output> {
    ShaderNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: index.handle().into(),
      level: SampleLevel::Auto,
      reference: None,
      offset: None,
    }
    .insert_api()
  }

  pub fn sample_index_level(
    &self,
    sampler: HandleNode<T::Sampler>,
    position: Node<T::Input>,
    index: Node<impl ShaderArrayTextureSampleIndexType>,
    level: Node<f32>,
  ) -> Node<T::Output> {
    ShaderNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: index.handle().into(),
      level: SampleLevel::Exact(level.handle()),
      reference: None,
      offset: None,
    }
    .insert_api()
  }
}

impl HandleNode<ShaderDepthTexture2DArray> {
  pub fn sample_compare_index_level(
    &self,
    sampler: HandleNode<ShaderCompareSampler>,
    position: Node<Vec2<f32>>,
    index: Node<impl ShaderArrayTextureSampleIndexType>,
    reference: Node<f32>,
    offset: Option<Vec2<i32>>,
  ) -> Node<f32> {
    ShaderNodeExpr::TextureSampling {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
      index: index.handle().into(),
      level: SampleLevel::Zero,
      reference: reference.handle().into(),
      offset,
    }
    .insert_api()
  }
}
