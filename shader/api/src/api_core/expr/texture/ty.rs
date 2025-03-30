use crate::*;

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

macro_rules! texture_dimension_impl {
  ($ty: tt, $ty_value: expr, $input_ty: tt) => {
    pub struct $ty;
    impl ShaderTextureDimension for $ty {
      const DIMENSION: TextureViewDimension = $ty_value;
      type Input<T> = $input_ty<T>;
    }
  };
}

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
impl DepthSampleTarget for TextureSampleDepth {}

pub struct MultiSampleOf<T>(T);
impl<T: ShaderTextureKind> ShaderTextureKind for MultiSampleOf<T> {
  const SAMPLING_TYPE: TextureSampleType = T::SAMPLING_TYPE;
  const IS_MULTI_SAMPLE: bool = true;
  type ChannelOutput = T::ChannelOutput;
}
impl<T> MultiSampleTarget for MultiSampleOf<T> {}
