use rendiation_webgpu::*;

use crate::*;

impl Scene {
  pub fn get_main_pass_load_op(&self) -> wgpu::Operations<wgpu::Color> {
    let load = if let Some(clear_color) = self.background.require_pass_clear() {
      wgpu::LoadOp::Clear(clear_color)
    } else {
      wgpu::LoadOp::Load
    };

    wgpu::Operations { load, store: true }
  }
}

pub struct ForwardScene;

impl PassContentWithSceneAndCamera for ForwardScene {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass, scene: &Scene, camera: &SceneCamera) {
    scene
      .models
      .iter()
      .for_each(|model| model.render(gpu, pass, &DefaultPassDispatcher, camera))
  }
}
