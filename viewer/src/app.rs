use winit::event::WindowEvent;

use crate::{renderer::Renderer, scene::Scene};

pub struct Application {
  scene: Scene,
}

impl Application {
  pub fn new() -> Self {
    let  scene = Scene::new();
    Self { scene }
  }

  pub fn render(&mut self, swap_chain_target: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    //
  }

  pub fn update(&mut self, event: WindowEvent) {
    //
  }
}
