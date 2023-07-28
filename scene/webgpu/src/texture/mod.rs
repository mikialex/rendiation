use crate::*;

mod cube;
mod d2;
mod pair;
mod sampler;

pub use cube::*;
pub use d2::*;
pub use pair::*;
pub use sampler::*;

#[derive(Clone)]
pub enum TextureGPUChange {
  Reference2D(GPU2DTextureView),
  ReferenceCube(GPUCubeTextureView),
  ReferenceSampler(GPUSamplerView),
  Content,
}

impl TextureGPUChange {
  fn into_render_component_delta(self) -> RenderComponentDeltaFlag {
    match self {
      TextureGPUChange::Reference2D(_) => RenderComponentDeltaFlag::ContentRef,
      TextureGPUChange::ReferenceCube(_) => RenderComponentDeltaFlag::ContentRef,
      TextureGPUChange::ReferenceSampler(_) => RenderComponentDeltaFlag::ContentRef,
      TextureGPUChange::Content => RenderComponentDeltaFlag::Content,
    }
  }
}

struct WebGPUTextureBackend;

impl GPUTextureBackend for WebGPUTextureBackend {
  type GPUTexture2D = GPU2DTextureView;
  type GPUSampler = GPUSamplerView;
  type GPUTexture2DBindingArray<const N: usize> = BindingResourceArray<GPU2DTextureView, N>;
  type GPUSamplerBindingArray<const N: usize> = BindingResourceArray<GPUSamplerView, N>;
  type BindingCollector = BindingBuilder;

  fn bind_texture2d(collector: &mut Self::BindingCollector, texture: &Self::GPUTexture2D) {
    collector.bind(texture);
  }

  fn bind_sampler(collector: &mut Self::BindingCollector, sampler: &Self::GPUSampler) {
    collector.bind(sampler);
  }

  fn bind_texture2d_array<const N: usize>(
    collector: &mut Self::BindingCollector,
    textures: &Self::GPUTexture2DBindingArray<N>,
  ) {
    collector.bind(textures);
  }

  fn bind_sampler_array<const N: usize>(
    collector: &mut Self::BindingCollector,
    samplers: &Self::GPUSamplerBindingArray<N>,
  ) {
    collector.bind(samplers);
  }
}
