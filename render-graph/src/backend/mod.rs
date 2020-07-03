use std::hash::Hash;

#[cfg(feature = "wgpu")]
pub mod webgpu;
use crate::RenderTargetFormatKey;
#[cfg(feature = "wgpu")]
pub use webgpu::*;

pub trait RenderGraphBackend: 'static {
  type RenderTarget: 'static;
  type RenderTargetFormatKey: Eq + Hash + Clone + Default + Sized;
  type Renderer;
  type RenderPass;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &RenderTargetFormatKey<Self::RenderTargetFormatKey>,
  ) -> Self::RenderTarget;

  fn dispose_render_target(renderer: &Self::Renderer, target: Self::RenderTarget);

  fn begin_render_pass(
    renderer: &mut Self::Renderer,
    target: &Self::RenderTarget,
  ) -> Self::RenderPass;
  fn end_render_pass(renderer: &Self::Renderer, pass: Self::RenderPass);
}
