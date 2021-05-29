use winit::event::WindowEvent;

use crate::{
  renderer::Renderer,
  scene::{OriginForward, RenderPassDispatcher, Scene},
};

pub struct Application {
  scene: Scene,
  origin: OriginForward,
}

impl Application {
  pub fn new() -> Self {
    let scene = Scene::new();
    Self {
      scene,
      origin: OriginForward,
    }
  }

  pub fn render(&mut self, frame: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    renderer.render(
      &mut RenderPassDispatcher {
        scene: &mut self.scene,
        style: &mut self.origin,
      },
      frame,
    )
  }

  pub fn update(&mut self, event: WindowEvent) {
    //
  }
}
