use crate::*;

// implementation notes: obviously we could leverage the const generics and if-hack bounds to do
// this, but the current implementation is more stable(not based on bunch of unstable features).

pub trait ShaderTextureDimension: 'static {
  const DIMENSION: TextureViewDimension;
  type Input<T>;
}
macro_rules! texture_dimension_impl {
  ($ty: tt, $ty_value: expr, $input_ty: tt) => {
    pub struct $ty;
    impl ShaderTextureDimension for $ty {
      const DIMENSION: TextureViewDimension = $ty_value;
      type Input<T> = $input_ty<T>;
    }
  };
}

pub trait ArrayLayerTarget {}
pub trait SingleLayerTarget {}

pub trait D1TextureType {}
pub trait D2LikeTextureType {}
pub trait D3TextureType {}

texture_dimension_impl!(TextureDimension1, TextureViewDimension::D1, Vec2);
impl SingleLayerTarget for TextureDimension1 {}
impl D1TextureType for TextureDimension1 {}
texture_dimension_impl!(TextureDimension2, TextureViewDimension::D2, Vec2);
impl SingleLayerTarget for TextureDimension2 {}
impl D2LikeTextureType for TextureDimension2 {}
texture_dimension_impl!(TextureDimension2Array, TextureViewDimension::D2Array, Vec2);
impl ArrayLayerTarget for TextureDimension2Array {}
impl D2LikeTextureType for TextureDimension2Array {}
texture_dimension_impl!(TextureDimensionCube, TextureViewDimension::Cube, Vec3);
impl SingleLayerTarget for TextureDimensionCube {}
impl D2LikeTextureType for TextureDimensionCube {}
texture_dimension_impl!(
  TextureDimensionCubeArray,
  TextureViewDimension::CubeArray,
  Vec3
);
impl ArrayLayerTarget for TextureDimensionCubeArray {}
impl D2LikeTextureType for TextureDimensionCubeArray {}
texture_dimension_impl!(TextureDimension3, TextureViewDimension::D3, Vec3);
impl SingleLayerTarget for TextureDimension3 {}
impl D3TextureType for TextureDimension3 {}

pub trait ShaderTextureKind: 'static {
  const SAMPLING_TYPE: TextureSampleType;
  const IS_MULTI_SAMPLE: bool;
  type ChannelOutput;
}
pub trait SingleSampleTarget {}
pub trait MultiSampleTarget {}

impl ShaderTextureKind for f32 {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Float { filterable: true };
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = Vec4<f32>;
}
impl SingleSampleTarget for f32 {}

impl ShaderTextureKind for u32 {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Uint;
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = Vec4<u32>;
}
impl SingleSampleTarget for u32 {}

impl ShaderTextureKind for i32 {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Sint;
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = Vec4<i32>;
}
impl SingleSampleTarget for i32 {}

pub struct TextureSampleDepth;
impl ShaderTextureKind for TextureSampleDepth {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Depth;
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = f32;
}
pub trait DepthSampleTarget {}
impl DepthSampleTarget for TextureSampleDepth {}

pub struct MultiSampleOf<T>(T);
impl<T: ShaderTextureKind> ShaderTextureKind for MultiSampleOf<T> {
  const SAMPLING_TYPE: TextureSampleType = T::SAMPLING_TYPE;
  const IS_MULTI_SAMPLE: bool = true;
  type ChannelOutput = T::ChannelOutput;
}
impl<T> MultiSampleTarget for MultiSampleOf<T> {}

pub struct ShaderTexture<D, F>(pub D, pub F);
impl<D, F> ShaderNodeSingleType for ShaderTexture<D, F>
where
  D: ShaderTextureDimension,
  F: ShaderTextureKind,
{
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::Texture {
      dimension: D::DIMENSION,
      sample_type: F::SAMPLING_TYPE,
      multi_sampled: F::IS_MULTI_SAMPLE,
    }
  }
}
impl<D, F> ShaderNodeType for ShaderTexture<D, F>
where
  D: ShaderTextureDimension,
  F: ShaderTextureKind,
{
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

// these are commonly used type

pub type ShaderTexture1D = ShaderTexture<TextureDimension1, f32>;
pub type ShaderTexture2D = ShaderTexture<TextureDimension2, f32>;
pub type ShaderTexture3D = ShaderTexture<TextureDimension3, f32>;

pub type ShaderTexture2DUint = ShaderTexture<TextureDimension2, u32>;

pub type ShaderTextureCube = ShaderTexture<TextureDimensionCube, f32>;
pub type ShaderTexture2DArray = ShaderTexture<TextureDimension2Array, f32>;
pub type ShaderTextureCubeArray = ShaderTexture<TextureDimensionCubeArray, f32>;

pub type ShaderDepthTexture2D = ShaderTexture<TextureDimension2, TextureSampleDepth>;
pub type ShaderDepthTextureCube = ShaderTexture<TextureDimensionCube, TextureSampleDepth>;
pub type ShaderDepthTexture2DArray = ShaderTexture<TextureDimension2Array, TextureSampleDepth>;
pub type ShaderDepthTextureCubeArray = ShaderTexture<TextureDimensionCubeArray, TextureSampleDepth>;

pub type ShaderMultiSampleTexture2D = ShaderTexture<TextureDimension2, MultiSampleOf<f32>>;
pub type ShaderMultiSampleDepthTexture2D =
  ShaderTexture<TextureDimension2, MultiSampleOf<TextureSampleDepth>>;

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
pub struct TextureSamplingAction<T> {
  tex: PhantomData<T>,
  info: ShaderTextureSampling,
}

impl<T> TextureSamplingAction<T> {
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
  pub fn with_level_bias(mut self, level: Node<f32>) -> Self {
    self.info.level = SampleLevel::Bias(level.handle());
    self
  }
  pub fn with_level_grad(
    mut self,
    x: Node<<Self as ShaderTextureDimension>::Input<f32>>,
    y: Node<<Self as ShaderTextureDimension>::Input<f32>>,
  ) -> Self
  where
    Self: ShaderTextureDimension,
  {
    self.info.level = SampleLevel::Gradient {
      x: x.handle(),
      y: y.handle(),
    };
    self
  }
  pub fn sample(self) -> Node<<Self as ShaderTextureKind>::ChannelOutput>
  where
    Self: ShaderTextureKind,
  {
    ShaderNodeExpr::TextureSampling(self.info).insert_api()
  }
  /// do texture gather, the level will be override as zero
  pub fn gather(mut self, channel: GatherChannel) -> Node<Vec4<f32>>
  where
    T: D2LikeTextureType,
  {
    // gather level can only be zero
    self.info.level = SampleLevel::Zero;
    self.info.gather_channel = Some(channel);
    ShaderNodeExpr::TextureSampling(self.info).insert_api()
  }
}

impl<T> BindingNode<T> {
  /// just for shortcut
  pub fn sample(
    &self,
    sampler: BindingNode<ShaderSampler>,
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
    sampler: BindingNode<ShaderSampler>,
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

  pub fn load_storage_texture_texel(&self, position: Node<T::LoadInput>) -> Node<T::Output>
  where
    T: SingleSampleTarget + SingleLayerTarget + ShaderDirectLoad,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: None,
      sample_index: None,
      level: None,
    })
    .insert_api()
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
      level: level.into_i32().handle().into(), // todo, fix naga require i32
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
      level: level.into_i32().handle().into(), // todo, fix naga require i32
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
    sampler: BindingNode<ShaderSampler>,
    position: impl Into<Node<T::Input>>,
  ) -> TextureSamplingAction<T>
  where
    T: SingleSampleTarget,
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

impl<T: DepthSampleTarget> BindingNode<T> {
  pub fn build_compare_sample_call(
    &self,
    sampler: BindingNode<ShaderCompareSampler>,
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
        level: SampleLevel::Zero,
        reference: reference.handle().into(),
        offset: None,
        gather_channel: None,
      },
    }
  }
}

impl<T> BindingNode<T> {
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
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumLevels).insert_api()
  }
}

impl<T> BindingNode<T> {
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
    T: D1TextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  /// using None means base level
  pub fn texture_dimension_2d(&self, level: Option<Node<u32>>) -> Node<Vec2<u32>>
  where
    T: D2LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  /// using None means base level
  pub fn texture_dimension_3d(&self, level: Option<Node<u32>>) -> Node<Vec3<u32>>
  where
    T: D3TextureType,
  {
    self.texture_dimension(level).insert_api()
  }
}

pub struct ShaderStorageTextureR1D;
pub struct ShaderStorageTextureRW1D;
pub struct ShaderStorageTextureW1D;

pub struct ShaderStorageTextureR2D;
pub struct ShaderStorageTextureRW2D;
pub struct ShaderStorageTextureW2D;

pub struct ShaderStorageTextureR3D;
pub struct ShaderStorageTextureRW3D;
pub struct ShaderStorageTextureW3D;

pub struct ShaderStorageTextureR2DArray;
pub struct ShaderStorageTextureRW2DArray;
pub struct ShaderStorageTextureW2DArray;

#[macro_export]
macro_rules! storage_tex_impl {
  ($ty: ty, $ty_value: expr) => {
    sg_node_impl!(
      $ty,
      ShaderValueSingleType::StorageTexture {
        dimension: $ty_value,
        format: StorageFormat::R8Unorm,
        access: StorageTextureAccess::Load,
      }
    );
  };
}

storage_tex_impl!(ShaderStorageTextureR1D, TextureViewDimension::D1);
storage_tex_impl!(ShaderStorageTextureRW1D, TextureViewDimension::D1);
storage_tex_impl!(ShaderStorageTextureW1D, TextureViewDimension::D1);

impl D1TextureType for ShaderStorageTextureR1D {}
impl D1TextureType for ShaderStorageTextureRW1D {}
impl D1TextureType for ShaderStorageTextureW1D {}

storage_tex_impl!(ShaderStorageTextureR2D, TextureViewDimension::D2);
storage_tex_impl!(ShaderStorageTextureRW2D, TextureViewDimension::D2);
storage_tex_impl!(ShaderStorageTextureW2D, TextureViewDimension::D2);

impl D2LikeTextureType for ShaderStorageTextureR2D {}
impl D2LikeTextureType for ShaderStorageTextureRW2D {}
impl D2LikeTextureType for ShaderStorageTextureW2D {}

storage_tex_impl!(ShaderStorageTextureR3D, TextureViewDimension::D3);
storage_tex_impl!(ShaderStorageTextureRW3D, TextureViewDimension::D3);
storage_tex_impl!(ShaderStorageTextureW3D, TextureViewDimension::D3);

impl D3TextureType for ShaderStorageTextureR3D {}
impl D3TextureType for ShaderStorageTextureRW3D {}
impl D3TextureType for ShaderStorageTextureW3D {}

storage_tex_impl!(ShaderStorageTextureR2DArray, TextureViewDimension::D2Array);
storage_tex_impl!(ShaderStorageTextureRW2DArray, TextureViewDimension::D2Array);
storage_tex_impl!(ShaderStorageTextureW2DArray, TextureViewDimension::D2Array);

impl D2LikeTextureType for ShaderStorageTextureR2DArray {}
impl D2LikeTextureType for ShaderStorageTextureRW2DArray {}
impl D2LikeTextureType for ShaderStorageTextureW2DArray {}

pub trait ShaderStorageTextureLike {}

impl ShaderStorageTextureLike for ShaderStorageTextureR1D {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW1D {}
impl ShaderStorageTextureLike for ShaderStorageTextureW1D {}

impl ShaderStorageTextureLike for ShaderStorageTextureR2D {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW2D {}
impl ShaderStorageTextureLike for ShaderStorageTextureW2D {}

impl ShaderStorageTextureLike for ShaderStorageTextureR3D {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW3D {}
impl ShaderStorageTextureLike for ShaderStorageTextureW3D {}

impl ShaderStorageTextureLike for ShaderStorageTextureR2DArray {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW2DArray {}
impl ShaderStorageTextureLike for ShaderStorageTextureW2DArray {}

impl SingleLayerTarget for ShaderStorageTextureR1D {}
impl SingleLayerTarget for ShaderStorageTextureRW1D {}
impl SingleLayerTarget for ShaderStorageTextureW1D {}

impl SingleLayerTarget for ShaderStorageTextureR2D {}
impl SingleLayerTarget for ShaderStorageTextureRW2D {}
impl SingleLayerTarget for ShaderStorageTextureW2D {}

impl SingleLayerTarget for ShaderStorageTextureR3D {}
impl SingleLayerTarget for ShaderStorageTextureRW3D {}
impl SingleLayerTarget for ShaderStorageTextureW3D {}

impl ArrayLayerTarget for ShaderStorageTextureR2DArray {}
impl ArrayLayerTarget for ShaderStorageTextureRW2DArray {}
impl ArrayLayerTarget for ShaderStorageTextureW2DArray {}

impl SingleSampleTarget for ShaderStorageTextureR1D {}
impl SingleSampleTarget for ShaderStorageTextureRW1D {}
impl SingleSampleTarget for ShaderStorageTextureW1D {}
impl SingleSampleTarget for ShaderStorageTextureR2D {}
impl SingleSampleTarget for ShaderStorageTextureRW2D {}
impl SingleSampleTarget for ShaderStorageTextureW2D {}
impl SingleSampleTarget for ShaderStorageTextureR3D {}
impl SingleSampleTarget for ShaderStorageTextureRW3D {}
impl SingleSampleTarget for ShaderStorageTextureW3D {}
impl SingleSampleTarget for ShaderStorageTextureR2DArray {}
impl SingleSampleTarget for ShaderStorageTextureRW2DArray {}
impl SingleSampleTarget for ShaderStorageTextureW2DArray {}

impl<T> BindingNode<T>
where
  T: ShaderTextureType + ShaderStorageTextureLike + ShaderDirectLoad + SingleLayerTarget,
{
  pub fn write_texel(&self, at: Node<T::LoadInput>, tex: Node<T::Output>) {
    call_shader_api(|api| {
      api.texture_store(ShaderTextureStore {
        image: self.handle(),
        position: at.handle(),
        array_index: None,
        value: tex.handle(),
      })
    })
  }
}

impl<T> BindingNode<T>
where
  T: ShaderTextureType + ShaderStorageTextureLike + ShaderDirectLoad + ArrayLayerTarget,
{
  pub fn write_texel_index(&self, at: Node<T::LoadInput>, index: Node<u32>, tex: Node<T::Output>) {
    call_shader_api(|api| {
      api.texture_store(ShaderTextureStore {
        image: self.handle(),
        position: at.handle(),
        array_index: Some(index.handle()),
        value: tex.handle(),
      })
    })
  }
}
