use crate::*;
use rendiation::*;

pub fn build_test_graph() {
  let graph: RenderGraph<WebGPURenderGraphBackend> = RenderGraph::new();
  let normal_pass = graph.pass("normal").viewport();
  let normal_target = graph.target("normal").from_pass(&normal_pass);
  let copy_screen = graph
    .pass("copy_screen")
    .viewport()
    .depend(&normal_target)
    .render_by(|_, _| {
      let _a = 1;
    });
  graph.finally().from_pass(&copy_screen);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WGPURenderTargetFormat {
  attachments: Vec<TextureFormat>, // not consider 3d texture stuff
  depth: Option<TextureFormat>,
}

impl Default for WGPURenderTargetFormat {
  fn default() -> Self {
    Self {
      attachments: vec![TextureFormat::Rgba8UnormSrgb],
      depth: Some(TextureFormat::Depth32Float),
    }
  }
}

pub struct WebGPURenderGraphBackend {}

impl RenderGraphBackend for WebGPURenderGraphBackend {
  type RenderTarget = RenderTarget;

  // can we use some enum stuff that cheaper?
  type RenderTargetFormatKey = WGPURenderTargetFormat;
  type Renderer = WGPURenderer;
  type RenderPass = WGPURenderPass<'static>;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &Self::RenderTargetFormatKey,
  ) -> Self::RenderTarget {
    todo!()
  }

  fn dispose_render_target(renderer: &Self::Renderer, target: Self::RenderTarget) {
    // just do target drop
  }

  fn begin_render_pass(renderer: &Self::Renderer, target: &Self::RenderTarget) -> Self::RenderPass {
    // target.create_render_pass_builder()
    // todo load op, store op...
  }
  fn end_render_pass(renderer: &Self::Renderer, pass: Self::RenderPass) {
    // just do pass drop
  }
}
