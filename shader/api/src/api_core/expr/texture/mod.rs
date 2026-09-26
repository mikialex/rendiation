use crate::*;

mod ty;
pub use ty::*;

mod sampling;
pub use sampling::*;

mod storage;
pub use storage::*;

// implementation notes: obviously we could leverage the const generics and if-hack bounds to do
// this, but the current implementation is more stable(not based on bunch of unstable features).

pub trait ShaderTextureDimension: 'static {
  const DIMENSION: TextureViewDimension;
  type Input<T>;
}

pub type TextureSampleInputOf<T, U> = <T as ShaderTextureDimension>::Input<U>;

pub trait ArrayLayerTarget: ShaderTextureDimension {}
pub trait SingleLayerTarget: ShaderTextureDimension {}

pub trait D1LikeTextureType: ShaderTextureDimension {}
pub trait D2LikeTextureType: ShaderTextureDimension {}
pub trait D3LikeTextureType: ShaderTextureDimension {}

pub trait ShaderTextureKind: 'static {
  const SAMPLING_TYPE: TextureSampleType;
  const IS_MULTI_SAMPLE: bool;
  type TexelOutput: ShaderSizedValueNodeType;
  type ChannelOutput: ShaderSizedValueNodeType;
}

pub type TexelOutputOf<T> = <T as ShaderTextureKind>::TexelOutput;
pub type ChannelOutputOf<T> = <T as ShaderTextureKind>::ChannelOutput;

pub trait DirectAccessTarget {}
pub trait SingleSampleTarget {}
pub trait MultiSampleTarget {}

pub trait DepthSampleTarget {}

/// The texture kind that could be sampled by the non comparison sampler(textureSample,
/// textureSampleLevel), which are the float and depth textures. The integer textures can only be
/// loaded or gathered.
pub trait SamplerSampleTarget: ShaderTextureKind + SingleSampleTarget {
  /// the level type of textureSampleLevel, f32 for float texture, u32 for depth texture
  type ExplicitLevel: ShaderNodeType;
}

/// The texture kind that support textureSampleBias and textureSampleGrad, which is the float
/// texture only.
pub trait SamplerBiasGradSampleTarget: SamplerSampleTarget {}

/// The texture dimension that support the sampling offset. WGSL also support 3d texture offset by
/// `vec3<i32>`, but it is not supported yet here.
pub trait TextureOffsetTarget: ShaderTextureDimension {}

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
