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
pub trait ShaderTextureType {
  type Input;
  type Output: PrimitiveShaderNodeType;
}

impl ShaderTextureType for ShaderTexture1D {
  type Input = f32;
  type Output = Vec4<f32>;
}

impl ShaderTextureType for ShaderTexture2D {
  type Input = Vec2<f32>;
  type Output = Vec4<f32>;
}

impl ShaderTextureType for ShaderDepthTexture2D {
  type Input = Vec2<f32>;
  type Output = f32;
}

impl ShaderTextureType for ShaderTexture3D {
  type Input = Vec3<f32>;
  type Output = Vec4<f32>;
}

impl ShaderTextureType for ShaderTextureCube {
  type Input = Vec3<f32>;
  type Output = Vec4<f32>;
}

impl ShaderTextureType for ShaderDepthTextureCube {
  type Input = Vec3<f32>;
  type Output = f32;
}

impl ShaderTextureType for ShaderTexture2DArray {
  type Input = Vec2<f32>;
  type Output = Vec4<f32>;
}
impl ShaderTextureType for ShaderTextureCubeArray {
  type Input = Vec3<f32>;
  type Output = Vec4<f32>;
}
impl ShaderTextureType for ShaderDepthTexture2DArray {
  type Input = Vec2<f32>;
  type Output = f32;
}
impl ShaderTextureType for ShaderDepthTextureCubeArray {
  type Input = Vec3<f32>;
  type Output = f32;
}

pub trait ArraySampleTarget {}
impl ArraySampleTarget for ShaderTexture2DArray {}
impl ArraySampleTarget for ShaderTextureCubeArray {}
impl ArraySampleTarget for ShaderDepthTexture2DArray {}
impl ArraySampleTarget for ShaderDepthTextureCubeArray {}

pub trait ShaderArrayTextureSampleIndexType: ShaderNodeType {}
impl ShaderArrayTextureSampleIndexType for u32 {}
impl ShaderArrayTextureSampleIndexType for i32 {}

pub trait DepthSampleTarget: ShaderTextureType<Output = f32> {}
impl DepthSampleTarget for ShaderDepthTexture2D {}
impl DepthSampleTarget for ShaderDepthTextureCube {}
impl DepthSampleTarget for ShaderDepthTexture2DArray {}
impl DepthSampleTarget for ShaderDepthTextureCubeArray {}

pub struct TextureSamplingAction<T> {
  tex: PhantomData<T>,
  info: ShaderTextureSampling,
}

impl<T: ShaderTextureType> TextureSamplingAction<T> {
  pub fn with_array_index(mut self, index: Node<impl ShaderArrayTextureSampleIndexType>) -> Self
  where
    T: ArraySampleTarget,
  {
    self.info.index = Some(index.handle());
    self
  }

  pub fn with_level(mut self, level: Node<f32>) -> Self {
    self.info.level = SampleLevel::Exact(level.handle());
    self
  }
  pub fn sample(self) -> Node<T::Output> {
    ShaderNodeExpr::TextureSampling(self.info).insert_api()
  }
}

impl<T: ShaderTextureType> HandleNode<T> {
  /// just for shortcut
  pub fn sample(
    &self,
    sampler: HandleNode<ShaderSampler>,
    position: impl Into<Node<T::Input>>,
  ) -> Node<T::Output> {
    self.build_sample_call(sampler, position).sample()
  }
  /// just for shortcut
  pub fn sample_zero_level(
    &self,
    sampler: HandleNode<ShaderSampler>,
    position: impl Into<Node<T::Input>>,
  ) -> Node<T::Output> {
    self
      .build_sample_call(sampler, position)
      .with_level(val(0.))
      .sample()
  }

  pub fn build_sample_call(
    &self,
    sampler: HandleNode<ShaderSampler>,
    position: impl Into<Node<T::Input>>,
  ) -> TextureSamplingAction<T> {
    TextureSamplingAction {
      tex: PhantomData,
      info: ShaderTextureSampling {
        texture: self.handle(),
        sampler: sampler.handle(),
        position: position.into().handle(),
        index: None,
        level: SampleLevel::Auto,
        reference: None,
        offset: None,
      },
    }
  }
}

pub struct DepthTextureSamplingAction<T> {
  tex: PhantomData<T>,
  info: ShaderTextureSampling,
}

impl<T> DepthTextureSamplingAction<T> {
  pub fn with_array_index(mut self, index: Node<impl ShaderArrayTextureSampleIndexType>) -> Self
  where
    T: ArraySampleTarget,
  {
    self.info.index = Some(index.handle());
    self
  }
  pub fn with_offset(mut self, offset: Vec2<i32>) -> Self {
    self.info.offset = Some(offset);
    self
  }
  pub fn with_zero_level(mut self) -> Self {
    self.info.level = SampleLevel::Zero;
    self
  }
  pub fn sample(self) -> Node<f32> {
    ShaderNodeExpr::TextureSampling(self.info).insert_api()
  }
}

impl<T: ShaderTextureType + DepthSampleTarget> HandleNode<T> {
  pub fn build_compare_sample_call(
    &self,
    sampler: HandleNode<ShaderCompareSampler>,
    position: impl Into<Node<T::Input>>,
    reference: Node<f32>,
  ) -> DepthTextureSamplingAction<T> {
    DepthTextureSamplingAction {
      tex: PhantomData,
      info: ShaderTextureSampling {
        texture: self.handle(),
        sampler: sampler.handle(),
        position: position.into().handle(),
        index: None,
        level: SampleLevel::Auto,
        reference: reference.handle().into(),
        offset: None,
      },
    }
  }
}
