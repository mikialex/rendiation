use rendiation_controller::{ControllerWinitAdapter, OrbitController};
use rendiation_texture::TextureSampler;
use winit::event::*;

use crate::{
  renderer::Renderer,
  scene::{RenderPassDispatcher, Scene, StandardForward},
};

pub struct Application {
  scene: Scene,
  origin: StandardForward,
  controller: ControllerWinitAdapter<OrbitController>,
}

impl Application {
  pub fn new() -> Self {
    let mut scene = Scene::new();

    let sampler = scene.add_sampler(TextureSampler::default());
    // let texture = scene.add_texture2d(todo!());

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    Self {
      scene,
      origin: StandardForward,
      controller,
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

  pub fn update(&mut self, event: &Event<()>) {
    self.controller.event(event)
    //
  }
}
