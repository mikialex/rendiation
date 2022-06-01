use crate::*;

pub fn get_main_pass_load_op(scene: &Scene<WebGPUScene>) -> webgpu::Operations<webgpu::Color> {
  let load = if let Some(clear_color) = scene.background.as_ref().unwrap().require_pass_clear() {
    webgpu::LoadOp::Clear(clear_color)
  } else {
    webgpu::LoadOp::Load
  };

  webgpu::Operations { load, store: true }
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
