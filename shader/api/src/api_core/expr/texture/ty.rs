use crate::*;

pub struct ShaderTexture<D, F>(pub D, pub F);
impl<D, F> ShaderNodeSingleType for ShaderTexture<D, F>
where
  D: ShaderTextureDimension,
  F: ShaderTextureKind,
  Self: ValidShaderTextureType,
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
  Self: ValidShaderTextureType,
{
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

macro_rules! texture_dimension_impl {
  ($ty: tt, $ty_value: expr, $input_ty: tt) => {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct $ty;
    impl ShaderTextureDimension for $ty {
      const DIMENSION: TextureViewDimension = $ty_value;
      type Input<T> = $input_ty<T>;
    }
  };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureDimension1;
impl ShaderTextureDimension for TextureDimension1 {
  const DIMENSION: TextureViewDimension = TextureViewDimension::D1;
  type Input<T> = T;
}
impl SingleLayerTarget for TextureDimension1 {}
impl D1LikeTextureType for TextureDimension1 {}
impl DirectAccessTarget for TextureDimension1 {}
impl StorageTextureDimension for TextureDimension1 {}

texture_dimension_impl!(TextureDimension2, TextureViewDimension::D2, Vec2);
impl SingleLayerTarget for TextureDimension2 {}
impl D2LikeTextureType for TextureDimension2 {}
impl DirectAccessTarget for TextureDimension2 {}
impl TextureOffsetTarget for TextureDimension2 {}
impl DepthTextureDimension for TextureDimension2 {}
impl StorageTextureDimension for TextureDimension2 {}

texture_dimension_impl!(TextureDimension2Array, TextureViewDimension::D2Array, Vec2);
impl ArrayLayerTarget for TextureDimension2Array {}
impl D2LikeTextureType for TextureDimension2Array {}
impl DirectAccessTarget for TextureDimension2Array {}
impl TextureOffsetTarget for TextureDimension2Array {}
impl DepthTextureDimension for TextureDimension2Array {}
impl StorageTextureDimension for TextureDimension2Array {}

texture_dimension_impl!(TextureDimensionCube, TextureViewDimension::Cube, Vec3);
impl SingleLayerTarget for TextureDimensionCube {}
impl D2LikeTextureType for TextureDimensionCube {}
impl DepthTextureDimension for TextureDimensionCube {}

texture_dimension_impl!(
  TextureDimensionCubeArray,
  TextureViewDimension::CubeArray,
  Vec3
);
impl ArrayLayerTarget for TextureDimensionCubeArray {}
impl D2LikeTextureType for TextureDimensionCubeArray {}
impl DepthTextureDimension for TextureDimensionCubeArray {}

texture_dimension_impl!(TextureDimension3, TextureViewDimension::D3, Vec3);
impl SingleLayerTarget for TextureDimension3 {}
impl D3LikeTextureType for TextureDimension3 {}
impl DirectAccessTarget for TextureDimension3 {}
impl StorageTextureDimension for TextureDimension3 {}

impl ShaderTextureKind for f32 {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Float { filterable: true };
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = f32;
  type TexelOutput = Vec4<f32>;
}
impl SingleSampleTarget for f32 {}
impl SamplerSampleTarget for f32 {
  type ExplicitLevel = f32;
}
impl SamplerBiasGradSampleTarget for f32 {}

impl ShaderTextureKind for u32 {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Uint;
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = u32;
  type TexelOutput = Vec4<u32>;
}
impl SingleSampleTarget for u32 {}

impl ShaderTextureKind for i32 {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Sint;
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = i32;
  type TexelOutput = Vec4<i32>;
}
impl SingleSampleTarget for i32 {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureSampleDepth;
impl ShaderTextureKind for TextureSampleDepth {
  const SAMPLING_TYPE: TextureSampleType = TextureSampleType::Depth;
  const IS_MULTI_SAMPLE: bool = false;
  type ChannelOutput = f32;
  type TexelOutput = f32;
}
impl DepthSampleTarget for TextureSampleDepth {}
impl SingleSampleTarget for TextureSampleDepth {}
impl SamplerSampleTarget for TextureSampleDepth {
  type ExplicitLevel = u32;
}

/// the inner kind must be single sampled, so the nested multi sample is not able to be expressed
pub struct MultiSampleOf<T>(T);
impl<T: ShaderTextureKind + SingleSampleTarget> ShaderTextureKind for MultiSampleOf<T> {
  const SAMPLING_TYPE: TextureSampleType = T::SAMPLING_TYPE;
  const IS_MULTI_SAMPLE: bool = true;
  type ChannelOutput = T::ChannelOutput;
  type TexelOutput = T::TexelOutput;
}
impl<T> MultiSampleTarget for MultiSampleOf<T> {}
