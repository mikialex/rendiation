use crate::*;

mod cube;
mod d2;
mod pair;
mod sampler;

pub use cube::*;
pub use d2::*;
pub use pair::*;
pub use sampler::*;

pub enum TextureGPUChange {
  Reference2D(GPU2DTextureView),
  ReferenceCube(GPUCubeTextureView),
  ReferenceSampler(GPUSamplerView),
  Content,
}

impl TextureGPUChange {
  fn to_render_component_delta(&self) -> RenderComponentDeltaFlag {
    match self {
      TextureGPUChange::Reference2D(_) => RenderComponentDeltaFlag::ContentRef,
      TextureGPUChange::ReferenceCube(_) => RenderComponentDeltaFlag::ContentRef,
      TextureGPUChange::ReferenceSampler(_) => RenderComponentDeltaFlag::ContentRef,
      TextureGPUChange::Content => RenderComponentDeltaFlag::ContentRef,
    }
  }
}
