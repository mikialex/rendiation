use std::hash::Hash;

#[cfg(feature = "wgpu")]
pub mod webgpu;
#[cfg(feature = "wgpu")]
pub use webgpu::*;

pub trait RenderGraphBackend {
  type RenderTarget: 'static;
  type RenderTargetFormatKey: Eq + Hash + Clone + Default;
  type Renderer;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &Self::RenderTargetFormatKey,
  ) -> Self::RenderTarget;

  fn dispose_render_target(renderer: &Self::Renderer, target: Self::RenderTarget);

  fn set_render_target(renderer: &Self::Renderer, target: &Self::RenderTarget);
  fn set_render_target_screen(renderer: &Self::Renderer);
}
