use crate::*;

mod cube;
mod d2;
mod pair;
mod sampler;

pub use cube::*;
pub use d2::*;
pub use pair::*;
pub use sampler::*;

/// note: we could design beautiful apis like Stream<Item = GPU2DTextureView> -> Stream<Item =
/// Texture2DHandle>, but for now, we require bindless downgrade ability, so we directly combined
/// the handle with the resource in BindableGPUChange
#[derive(Clone)]
pub enum BindableGPUChange {
  Reference2D(GPU2DTextureView, Texture2DHandle),
  ReferenceCube(GPUCubeTextureView),
  ReferenceSampler(GPUSamplerView, SamplerHandle),
  Content,
}

impl BindableGPUChange {
  fn into_render_component_delta(self) -> RenderComponentDeltaFlag {
    match self {
      BindableGPUChange::Reference2D(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::ReferenceCube(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::ReferenceSampler(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::Content => RenderComponentDeltaFlag::Content,
    }
  }
}
