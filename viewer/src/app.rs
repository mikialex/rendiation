use winit::event::WindowEvent;

use crate::{renderer::Renderer, scene::Scene};

pub struct Application {
  scene: Scene,
}

impl Application {
  pub fn new() -> Self {
    let scene = Scene::new();
    Self { scene }
  }

  pub fn render(&mut self, frame: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    renderer.render(&mut self.scene, frame)
  }

  pub fn update(&mut self, event: WindowEvent) {
    //
  }
}
