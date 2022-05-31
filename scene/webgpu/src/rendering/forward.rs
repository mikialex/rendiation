use crate::*;

impl Scene<WebGPUScene> {
  pub fn get_main_pass_load_op(&self) -> wgpu::Operations<wgpu::Color> {
    let load = if let Some(clear_color) = self.background.as_ref().unwrap().require_pass_clear() {
      wgpu::LoadOp::Clear(clear_color)
    } else {
      wgpu::LoadOp::Load
    };

    wgpu::Operations { load, store: true }
  }
}

pub struct ForwardScene;

impl PassContentWithSceneAndCamera for ForwardScene {
  fn render(
    &mut self,
    pass: &mut SceneRenderPass,
    scene: &Scene<WebGPUScene>,
    camera: &SceneCamera,
  ) {
    scene
      .models
      .iter()
      .for_each(|model| model.render(pass, &pass.default_dispatcher(), camera))
  }
}
