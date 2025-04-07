use crate::*;

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

pub trait ShaderArrayTextureSampleIndexType: ShaderNodeType {}
impl ShaderArrayTextureSampleIndexType for u32 {}
impl ShaderArrayTextureSampleIndexType for i32 {}

#[derive(Clone, Copy)]
pub struct TextureSamplingAction<D, F> {
  tex: PhantomData<(D, F)>,
  info: ShaderTextureSampling,
}

impl<D, F> TextureSamplingAction<D, F>
where
  D: ShaderTextureDimension,
  F: ShaderTextureKind,
{
  pub fn with_array_index(mut self, index: Node<impl ShaderArrayTextureSampleIndexType>) -> Self
  where
    D: ArrayLayerTarget,
  {
    self.info.array_index = Some(index.handle());
    self
  }

  pub fn with_level(mut self, level: Node<f32>) -> Self {
    self.info.level = SampleLevel::Exact(level.handle());
    self
  }

  pub fn with_level_bias(mut self, level: Node<f32>) -> Self {
    self.info.level = SampleLevel::Bias(level.handle());
    self
  }

  pub fn with_level_grad(
    mut self,
    x: Node<TextureSampleInputOf<D, f32>>,
    y: Node<TextureSampleInputOf<D, f32>>,
  ) -> Self {
    self.info.level = SampleLevel::Gradient {
      x: x.handle(),
      y: y.handle(),
    };
    self
  }
  pub fn sample(self) -> Node<ChannelOutputOf<F>> {
    ShaderNodeExpr::TextureSampling(self.info).insert_api()
  }
  /// do texture gather, the level will be override as zero
  pub fn gather(mut self, channel: GatherChannel) -> Node<Vec4<f32>>
  where
    D: D2LikeTextureType,
  {
    // gather level can only be zero
    self.info.level = SampleLevel::Zero;
    self.info.gather_channel = Some(channel);
    ShaderNodeExpr::TextureSampling(self.info).insert_api()
  }
}

impl<D: ShaderTextureDimension, F: ShaderTextureKind> BindingNode<ShaderTexture<D, F>> {
  /// just for shortcut
  pub fn sample(
    &self,
    sampler: BindingNode<ShaderSampler>,
    position: impl Into<Node<TextureSampleInputOf<D, f32>>>,
  ) -> Node<ChannelOutputOf<F>>
  where
    F: SingleSampleTarget,
  {
    self.build_sample_call(sampler, position).sample()
  }
  /// just for shortcut
  pub fn sample_zero_level(
    &self,
    sampler: BindingNode<ShaderSampler>,
    position: impl Into<Node<TextureSampleInputOf<D, f32>>>,
  ) -> Node<ChannelOutputOf<F>>
  where
    F: SingleSampleTarget,
  {
    self
      .build_sample_call(sampler, position)
      .with_level(val(0.))
      .sample()
  }

  pub fn load_texel(
    &self,
    position: Node<TextureSampleInputOf<D, u32>>,
    level: Node<u32>,
  ) -> Node<ChannelOutputOf<F>>
  where
    F: SingleSampleTarget,
    D: SingleLayerTarget + DirectAccessTarget,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: None,
      sample_index: None,
      level: level.into_i32().handle().into(), // todo, fix naga require i32
    })
    .insert_api()
  }

  pub fn load_texel_layer(
    &self,
    position: Node<TextureSampleInputOf<D, u32>>,
    layer: Node<u32>,
    level: Node<u32>,
  ) -> Node<ChannelOutputOf<F>>
  where
    D: ArrayLayerTarget + DirectAccessTarget,
    F: SingleSampleTarget,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: layer.handle().into(),
      sample_index: None,
      level: level.into_i32().handle().into(), // todo, fix naga require i32
    })
    .insert_api()
  }

  pub fn load_texel_multi_sample_index(
    &self,
    position: Node<TextureSampleInputOf<D, u32>>,
    sample_index: Node<u32>,
  ) -> Node<ChannelOutputOf<F>>
  where
    F: MultiSampleTarget,
    D: SingleLayerTarget + DirectAccessTarget,
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
    sampler: BindingNode<ShaderSampler>,
    position: impl Into<Node<TextureSampleInputOf<D, f32>>>,
  ) -> TextureSamplingAction<D, F>
  where
    F: SingleSampleTarget,
  {
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
        gather_channel: None,
      },
    }
  }
}

#[derive(Clone, Copy)]
pub struct DepthTextureSamplingAction<D, F> {
  tex: PhantomData<(D, F)>,
  info: ShaderTextureSampling,
}

impl<D, F> DepthTextureSamplingAction<D, F> {
  pub fn with_array_index(mut self, index: Node<impl ShaderArrayTextureSampleIndexType>) -> Self
  where
    D: ArrayLayerTarget,
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

impl<D: ShaderTextureDimension, F: ShaderTextureKind> BindingNode<ShaderTexture<D, F>> {
  pub fn build_compare_sample_call(
    &self,
    sampler: BindingNode<ShaderCompareSampler>,
    position: impl Into<Node<TextureSampleInputOf<D, f32>>>,
    reference: Node<f32>,
  ) -> DepthTextureSamplingAction<D, F> {
    DepthTextureSamplingAction {
      tex: PhantomData,
      info: ShaderTextureSampling {
        texture: self.handle(),
        sampler: sampler.handle(),
        position: position.into().handle(),
        array_index: None,
        level: SampleLevel::Zero,
        reference: reference.handle().into(),
        offset: None,
        gather_channel: None,
      },
    }
  }
}

impl<D: ShaderTextureDimension, F> BindingNode<ShaderTexture<D, F>> {
  pub fn texture_number_samples(&self) -> Node<u32>
  where
    F: MultiSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumSamples).insert_api()
  }
  pub fn texture_number_layers(&self) -> Node<u32>
  where
    D: ArrayLayerTarget + SingleSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumLayers).insert_api()
  }
  pub fn texture_number_levels(&self) -> Node<u32>
  where
    F: SingleSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumLevels).insert_api()
  }

  /// using None means base level
  fn texture_dimension(&self, level: Option<Node<u32>>) -> ShaderNodeExpr {
    ShaderNodeExpr::TextureQuery(
      self.handle(),
      TextureQuery::Size {
        level: level.map(|v| v.handle()),
      },
    )
  }

  /// using None means base level
  pub fn texture_dimension_1d(&self, level: Option<Node<u32>>) -> Node<u32>
  where
    D: D1LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  /// using None means base level
  pub fn texture_dimension_2d(&self, level: Option<Node<u32>>) -> Node<Vec2<u32>>
  where
    D: D2LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  /// using None means base level
  pub fn texture_dimension_3d(&self, level: Option<Node<u32>>) -> Node<Vec3<u32>>
  where
    D: D3LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }
}
