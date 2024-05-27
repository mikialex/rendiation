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
pub struct ShaderMultiSampleTexture2D;
#[derive(Clone, Copy)]
pub struct ShaderMultiSampleDepthTexture2D;

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
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderTextureCube,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Float { filterable: true },
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderTexture1D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D1,
    sample_type: TextureSampleType::Float { filterable: true },
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderTexture3D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D3,
    sample_type: TextureSampleType::Float { filterable: true },
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderTexture2DArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Float { filterable: true },
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderTextureCubeArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Float { filterable: true },
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderDepthTexture2D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Depth,
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderDepthTexture2DArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2Array,
    sample_type: TextureSampleType::Depth,
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderDepthTextureCube,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::Cube,
    sample_type: TextureSampleType::Depth,
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderDepthTextureCubeArray,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::CubeArray,
    sample_type: TextureSampleType::Depth,
    multi_sampled: false,
  }
);
sg_node_impl!(
  ShaderMultiSampleTexture2D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Float { filterable: true },
    multi_sampled: true,
  }
);

sg_node_impl!(
  ShaderMultiSampleDepthTexture2D,
  ShaderValueSingleType::Texture {
    dimension: TextureViewDimension::D2,
    sample_type: TextureSampleType::Depth,
    multi_sampled: true,
  }
);

/// https://www.w3.org/TR/WGSL/#texturesample
pub trait ShaderTextureType {
  type Input;
  type Output: PrimitiveShaderNodeType;
}

pub trait ShaderDirectLoad: ShaderTextureType {
  type LoadInput;
}

impl ShaderTextureType for ShaderTexture1D {
  type Input = f32;
  type Output = Vec4<f32>;
}
impl ShaderDirectLoad for ShaderTexture1D {
  type LoadInput = u32;
}

impl ShaderTextureType for ShaderTexture2D {
  type Input = Vec2<f32>;
  type Output = Vec4<f32>;
}
impl ShaderDirectLoad for ShaderTexture2D {
  type LoadInput = Vec2<u32>;
}

impl ShaderTextureType for ShaderDepthTexture2D {
  type Input = Vec2<f32>;
  type Output = f32;
}
impl ShaderDirectLoad for ShaderDepthTexture2D {
  type LoadInput = Vec2<u32>;
}

impl ShaderTextureType for ShaderTexture3D {
  type Input = Vec3<f32>;
  type Output = Vec4<f32>;
}
impl ShaderDirectLoad for ShaderTexture3D {
  type LoadInput = Vec3<u32>;
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
impl ShaderDirectLoad for ShaderTexture2DArray {
  type LoadInput = Vec2<u32>;
}
impl ShaderTextureType for ShaderTextureCubeArray {
  type Input = Vec3<f32>;
  type Output = Vec4<f32>;
}
impl ShaderTextureType for ShaderDepthTexture2DArray {
  type Input = Vec2<f32>;
  type Output = f32;
}
impl ShaderDirectLoad for ShaderDepthTexture2DArray {
  type LoadInput = Vec2<u32>;
}
impl ShaderTextureType for ShaderDepthTextureCubeArray {
  type Input = Vec3<f32>;
  type Output = f32;
}

impl ShaderTextureType for ShaderMultiSampleTexture2D {
  type Input = Vec2<f32>;
  type Output = Vec4<f32>;
}
impl ShaderDirectLoad for ShaderMultiSampleTexture2D {
  type LoadInput = Vec2<u32>;
}
impl ShaderTextureType for ShaderMultiSampleDepthTexture2D {
  type Input = Vec2<f32>;
  type Output = f32;
}
impl ShaderDirectLoad for ShaderMultiSampleDepthTexture2D {
  type LoadInput = Vec2<u32>;
}

pub trait ArrayLayerTarget {}
impl ArrayLayerTarget for ShaderTexture2DArray {}
impl ArrayLayerTarget for ShaderTextureCubeArray {}
impl ArrayLayerTarget for ShaderDepthTexture2DArray {}
impl ArrayLayerTarget for ShaderDepthTextureCubeArray {}

pub trait SingleLayerTarget {}
impl SingleLayerTarget for ShaderTexture1D {}
impl SingleLayerTarget for ShaderTexture2D {}
impl SingleLayerTarget for ShaderTexture3D {}
impl SingleLayerTarget for ShaderTextureCube {}
impl SingleLayerTarget for ShaderDepthTexture2D {}
impl SingleLayerTarget for ShaderDepthTextureCube {}

pub trait SingleSampleTarget {}
impl SingleSampleTarget for ShaderTexture1D {}
impl SingleSampleTarget for ShaderTexture2D {}
impl SingleSampleTarget for ShaderTexture3D {}
impl SingleSampleTarget for ShaderTextureCube {}
impl SingleSampleTarget for ShaderTexture2DArray {}
impl SingleSampleTarget for ShaderTextureCubeArray {}
impl SingleSampleTarget for ShaderDepthTexture2D {}
impl SingleSampleTarget for ShaderDepthTextureCube {}
impl SingleSampleTarget for ShaderDepthTexture2DArray {}
impl SingleSampleTarget for ShaderDepthTextureCubeArray {}

pub trait MultiSampleTarget {}
impl ArrayLayerTarget for ShaderMultiSampleTexture2D {}
impl ArrayLayerTarget for ShaderMultiSampleDepthTexture2D {}

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
    T: ArrayLayerTarget,
  {
    self.info.array_index = Some(index.handle());
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
  ) -> Node<T::Output>
  where
    T: SingleSampleTarget,
  {
    self.build_sample_call(sampler, position).sample()
  }
  /// just for shortcut
  pub fn sample_zero_level(
    &self,
    sampler: HandleNode<ShaderSampler>,
    position: impl Into<Node<T::Input>>,
  ) -> Node<T::Output>
  where
    T: SingleSampleTarget,
  {
    self
      .build_sample_call(sampler, position)
      .with_level(val(0.))
      .sample()
  }

  pub fn load_texel(&self, position: Node<T::LoadInput>, level: Node<u32>) -> Node<T::Output>
  where
    T: SingleSampleTarget + SingleLayerTarget + ShaderDirectLoad,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: None,
      sample_index: None,
      level: level.handle().into(),
    })
    .insert_api()
  }

  pub fn load_texel_layer(
    &self,
    position: Node<T::LoadInput>,
    layer: Node<u32>,
    level: Node<u32>,
  ) -> Node<T::Output>
  where
    T: SingleSampleTarget + ArrayLayerTarget + ShaderDirectLoad,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: layer.handle().into(),
      sample_index: None,
      level: level.handle().into(),
    })
    .insert_api()
  }

  /// note, level can not be dynamically decided
  pub fn load_texel_multi_sample_index(
    &self,
    position: Node<T::LoadInput>,
    sample_index: Node<u32>,
  ) -> Node<T::Output>
  where
    T: MultiSampleTarget + ShaderDirectLoad,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: None,
      sample_index: sample_index.handle().into(),
      level: None,
    })
    .insert_api()
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
        array_index: None,
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
    T: ArrayLayerTarget,
  {
    self.info.array_index = Some(index.handle());
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
        array_index: None,
        level: SampleLevel::Auto,
        reference: reference.handle().into(),
        offset: None,
      },
    }
  }
}

impl<T: ShaderTextureType> HandleNode<T> {
  pub fn texture_number_samples(&self) -> Node<u32>
  where
    T: MultiSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumSamples).insert_api()
  }
  pub fn texture_number_layers(&self) -> Node<u32>
  where
    T: ArrayLayerTarget + SingleSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumLayers).insert_api()
  }
  pub fn texture_number_levels(&self) -> Node<u32>
  where
    T: SingleSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumLayers).insert_api()
  }
}

pub trait D1TextureType {}
impl D1TextureType for ShaderTexture1D {}
pub trait D2LikeTextureType {}
impl D2LikeTextureType for ShaderTexture2D {}
impl D2LikeTextureType for ShaderTextureCube {}
impl D2LikeTextureType for ShaderTexture2DArray {}
impl D2LikeTextureType for ShaderTextureCubeArray {}
impl D2LikeTextureType for ShaderDepthTexture2D {}
impl D2LikeTextureType for ShaderDepthTextureCube {}
impl D2LikeTextureType for ShaderDepthTexture2DArray {}
impl D2LikeTextureType for ShaderDepthTextureCubeArray {}
impl D2LikeTextureType for ShaderMultiSampleTexture2D {}
impl D2LikeTextureType for ShaderMultiSampleDepthTexture2D {}
pub trait D3TextureType {}
impl D3TextureType for ShaderTexture3D {}

impl<T: ShaderTextureType> HandleNode<T> {
  fn texture_dimension(&self, level: Node<u32>) -> ShaderNodeExpr {
    ShaderNodeExpr::TextureQuery(
      self.handle(),
      TextureQuery::Size {
        level: Some(level.handle()),
      },
    )
  }

  pub fn texture_dimension_1d(&self, level: Node<u32>) -> Node<u32>
  where
    T: D1TextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  pub fn texture_dimension_2d(&self, level: Node<u32>) -> Node<Vec2<u32>>
  where
    T: D2LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  pub fn texture_dimension_3d(&self, level: Node<u32>) -> Node<Vec3<u32>>
  where
    T: D3TextureType,
  {
    self.texture_dimension(level).insert_api()
  }
}
