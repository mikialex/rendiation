use crate::TargetNodeData;
use std::hash::Hash;

pub trait RenderGraphBackend {
  type RenderTarget: 'static;
  type RenderTargetFormatKey: Eq + Hash + Copy;
  type Renderer;

  fn to_format_key(target_info: &TargetNodeData) -> Self::RenderTargetFormatKey;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &Self::RenderTargetFormatKey,
  ) -> Self::RenderTarget;

  fn dispose_render_target(renderer: &Self::Renderer, target: Self::RenderTarget);

  fn set_render_target(renderer: &Self::Renderer, target: &Self::RenderTarget);
  fn set_render_target_screen(renderer: &Self::Renderer);
}
