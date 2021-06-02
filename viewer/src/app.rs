use winit::event::WindowEvent;

use crate::{
  renderer::Renderer,
  scene::{RenderPassDispatcher, Scene, StandardForward},
};

pub struct Application {
  scene: Scene,
  origin: StandardForward,
}

impl Application {
  pub fn new() -> Self {
    let scene = Scene::new();
    Self {
      scene,
      origin: StandardForward,
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
