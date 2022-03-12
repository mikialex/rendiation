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

pub struct ForwardScene<'a> {
  scene: &'a mut Scene,
}

impl<'a> PassContent for ForwardScene<'a> {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass) {
    self.scene.models.iter().for_each(|model| {
      model.setup_pass(
        gpu,
        &mut pass,
        &DefaultPassDispatcher,
        self.scene.active_camera.as_ref().unwrap(),
      )
    })
  }
}
