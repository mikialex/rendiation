use crate::*;
use rendiation_webgpu::*;

pub fn build_test_graph() {
  let graph: RenderGraph<WebGPURenderGraphBackend> = RenderGraph::new();
  let normal_pass = graph.pass("normal");
  let normal_target = graph.target("normal").from_pass(&normal_pass);
  let copy_screen = graph
    .pass("copy_screen")
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
  type RenderTarget = Box<dyn RenderTargetAble>;

  type RenderTargetFormatKey = WGPURenderTargetFormat;
  type Renderer = WGPURenderer;
  type RenderPassBuilder = WGPURenderPassBuilder<'static>;
  type RenderPass = WGPURenderPass<'static>; // this need unbound lifetime

  fn create_render_target(
    renderer: &Self::Renderer,
    key: &RenderTargetFormatKey<Self::RenderTargetFormatKey>,
  ) -> Self::RenderTarget {
    Box::new(key.format.create_render_target(renderer, key.size))
  }

  fn dispose_render_target(_: &Self::Renderer, _: Self::RenderTarget) {
    // just do target drop
  }

  fn create_render_pass_builder(
    _: &Self::Renderer,
    target: &Self::RenderTarget,
  ) -> Self::RenderPassBuilder {
    let builder = target.create_render_pass_builder();
    unsafe { std::mem::transmute(builder) }
  }

  fn begin_render_pass(
    renderer: &mut Self::Renderer,
    builder: Self::RenderPassBuilder,
  ) -> Self::RenderPass {
    let pass = builder.create(renderer);
    unsafe { std::mem::transmute(pass) }
  }
  fn end_render_pass(_: &Self::Renderer, _: Self::RenderPass) {
    // just do pass drop
  }

  fn get_target_size(target: &Self::RenderTarget) -> RenderTargetSize {
    let size = target.get_size();
    RenderTargetSize::new(size.0, size.1)
  }
  fn set_viewport(_: &Self::Renderer, pass: &mut Self::RenderPass, viewport: Viewport) {
    pass.use_viewport(&viewport);
  }
}
