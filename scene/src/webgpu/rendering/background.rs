use rendiation_webgpu::*;

use crate::*;

pub struct BackGroundRendering<'a> {
  scene: &'a mut Scene,
}

impl<'a> PassContent for BackGroundRendering<'a> {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass) {
    self.scene.background.setup_pass(
      gpu,
      pass,
      &DefaultPassDispatcher,
      self.scene.active_camera.as_ref().unwrap(),
    );
  }
}
