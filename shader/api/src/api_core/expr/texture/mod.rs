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
  type ChannelOutput: ShaderSizedValueNodeType;
}

pub type ChannelOutputOf<T> = <T as ShaderTextureKind>::ChannelOutput;

pub trait DirectAccessTarget {}
pub trait SingleSampleTarget {}
pub trait MultiSampleTarget {}

pub trait DepthSampleTarget {}

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
