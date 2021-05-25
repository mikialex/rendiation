use winit::event::WindowEvent;

use crate::{
  renderer::Renderer,
  scene::{Scene, SceneResource},
};

pub struct Application {
  scene: Scene,
  resource: SceneResource,
}

impl Application {
  pub fn new() -> Self {
    let scene = Scene::new();
    Self {
      scene,
      resource: SceneResource::new(),
    }
  }

  pub fn render(&mut self, frame: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    renderer.render(&mut self.scene, frame, &mut self.resource)
  }

  pub fn update(&mut self, event: WindowEvent) {
    //
  }
}
