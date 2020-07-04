use crate::{RenderTargetFormatKey, RenderTargetSize};
use rendiation_render_entity::Viewport;
use std::hash::Hash;

#[cfg(feature = "webgpu")]
pub mod webgpu;
#[cfg(feature = "webgpu")]
pub use webgpu::*;

pub trait RenderGraphBackend: 'static {
  type RenderTarget: 'static;
  type RenderTargetFormatKey: Eq + Hash + Clone + Default + Sized;
  type Renderer;
  type RenderPassBuilder;
  type RenderPass;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &RenderTargetFormatKey<Self::RenderTargetFormatKey>,
  ) -> Self::RenderTarget;

  fn dispose_render_target(renderer: &Self::Renderer, target: Self::RenderTarget);

  fn create_render_pass_builder(
    renderer: &Self::Renderer,
    target: &Self::RenderTarget,
  ) -> Self::RenderPassBuilder;

  fn begin_render_pass(
    renderer: &mut Self::Renderer,
    builder: Self::RenderPassBuilder,
  ) -> Self::RenderPass;
  fn end_render_pass(renderer: &Self::Renderer, pass: Self::RenderPass);

  fn get_target_size(target: &Self::RenderTarget) -> RenderTargetSize;
  fn set_viewport(renderer: &Self::Renderer, pass: &mut Self::RenderPass, viewport: Viewport);
}
