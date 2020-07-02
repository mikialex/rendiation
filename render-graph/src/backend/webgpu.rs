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

impl WGPURenderTargetFormat {
  fn create_render_target(&self, renderer: &WGPURenderer, size: RenderTargetSize) -> RenderTarget {
    RenderTarget::new(
      self
        .attachments
        .iter()
        .map(|a| WGPUTexture::new_as_depth(renderer, *a, size.to_tuple()))
        .collect(),
      self
        .depth
        .map(|d| WGPUTexture::new_as_depth(renderer, d, size.to_tuple())),
    )
  }
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

  type RenderTargetFormatKey = WGPURenderTargetFormat;
  type Renderer = WGPURenderer;
  type RenderPass = WGPURenderPass<'static>;

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &RenderTargetFormatKey<Self::RenderTargetFormatKey>,
  ) -> Self::RenderTarget {
    key.format.create_render_target(renderer, key.size)
  }

  fn dispose_render_target(_: &Self::Renderer, _: Self::RenderTarget) {
    // just do target drop
  }

  fn begin_render_pass(
    renderer: &mut Self::Renderer,
    target: &Self::RenderTarget,
  ) -> Self::RenderPass {
    // target.create_render_pass_builder()
    // todo load op, store op...
    let builder = target.create_render_pass_builder();
    let pass = builder.create(&mut renderer.encoder);
    unsafe { std::mem::transmute(pass) }
  }
  fn end_render_pass(_: &Self::Renderer, _: Self::RenderPass) {
    // just do pass drop
  }
}
