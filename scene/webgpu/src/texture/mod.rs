use crate::*;

mod cube;
mod d2;
mod pair;

pub use cube::*;
pub use d2::*;
pub use pair::*;

pub enum TextureGPUChange {
  Reference2D(GPU2DTextureView),
  ReferenceCube(GPUCubeTextureView),
  Content,
}

impl TextureGPUChange {
  fn to_render_component_delta(&self) -> RenderComponentDelta {
    match self {
      TextureGPUChange::Reference2D(_) => RenderComponentDelta::ContentRef,
      TextureGPUChange::ReferenceCube(_) => RenderComponentDelta::ContentRef,
      TextureGPUChange::Content => RenderComponentDelta::ContentRef,
    }
  }
}
