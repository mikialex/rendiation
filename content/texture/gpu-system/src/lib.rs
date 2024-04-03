// we design this crate to provide an abstraction over different global gpu texture management
// strategy

use reactive::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod system;
pub use system::*;
mod bindless;
pub use bindless::*;
mod gles;
pub use gles::*;

pub type Texture2DHandle = u32;
pub type SamplerHandle = u32;

pub trait AbstractTraditionalTextureSystem {
  fn bind_texture2d(&mut self, collector: &mut BindingBuilder, handle: Texture2DHandle);
  fn bind_sampler(&mut self, collector: &mut BindingBuilder, handle: SamplerHandle);

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D>;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler>;
}

pub trait AbstractIndirectGPUTextureSystem {
  fn bind_system_self(&mut self, collector: &mut BindingBuilder);
  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder);
  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}
