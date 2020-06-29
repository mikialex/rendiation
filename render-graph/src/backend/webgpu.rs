use crate::{RenderGraphBackend, RenderGraph};
use rendiation::*;

pub fn build_test_graph() {
  let graph: RenderGraph<WebGPURenderGraphBackend>= RenderGraph::new();
  let normal_pass = graph.pass("normal").viewport();
  let normal_target = graph.target("normal").from_pass(&normal_pass);
  let copy_screen = graph
    .pass("copy_screen")
    .viewport()
    .depend(&normal_target)
    .render_by(|| {
      let _a = 1;
    });
  graph.screen().from_pass(&copy_screen);
}



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WGPURenderTargetFormat{
  attachments: Vec<TextureFormat>, // not consider 3d texture stuff
  depth: Option<TextureFormat>,
}

impl Default for WGPURenderTargetFormat{
  fn default() -> Self { unimplemented!() }
}

pub struct WebGPURenderGraphBackend {}

impl RenderGraphBackend for WebGPURenderGraphBackend {
  type RenderTarget = RenderTarget;
  type RenderTargetFormatKey = WGPURenderTargetFormat; // improve , can we use some enum stuff that cheaper?
  type Renderer = WGPURenderer;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &Self::RenderTargetFormatKey,
  ) -> Self::RenderTarget {
    todo!()
  }

  fn dispose_render_target(renderer: &Self::Renderer, target: Self::RenderTarget) {
    todo!()
  }

  fn set_render_target(renderer: &Self::Renderer, target: &Self::RenderTarget) {
    todo!()
  }
  fn set_render_target_screen(renderer: &Self::Renderer) {
    todo!()
  }
}
