use crate::rinecraft::RinecraftState;
use rendiation_ral::ResourceManager;
use rendiation_scenegraph::{default_impl::DefaultSceneBackend, DrawcallList, Scene};
use rendiation_webgpu::{
  renderer::SwapChain, RenderTargetAble, ScreenRenderTarget, ScreenRenderTargetInstance,
  WGPURenderer,
};
use rendium::EventCtx;

pub struct RinecraftRenderer {}

impl RinecraftRenderer {
  pub fn new() -> Self {
    Self {}
  }

  pub fn render(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WGPURenderer>,
    resource: &mut ResourceManager<WGPURenderer>,
    output: &ScreenRenderTargetInstance,
  ) {
    let list = scene.update(resource);
    resource.maintain_gpu(renderer);

    {
      let mut pass = output
        .create_render_pass_builder()
        .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
        .depth(|d| d.load_with_clear(1.0).ok())
        .create(&mut renderer.encoder);

      list.render(unsafe { std::mem::transmute(&mut pass) }, scene, resource);
    }

    renderer
      .queue
      .submit(&renderer.device, &mut renderer.encoder);
  }
}
