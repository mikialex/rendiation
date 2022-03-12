use rendiation_webgpu::*;

use crate::*;

pub struct BackGroundRendering<'a> {
  scene: &'a mut Scene,
}

impl<'a> PassContent for BackGroundRendering<'a> {
  fn render(&mut self, gpu: &GPU, pass: &mut GPURenderPass) {
    let mut pass = SceneRenderPass {
      pass,
      dispatcher: &DefaultPassDispatcher,
      binding: Default::default(),
    };
    self
      .scene
      .background
      .setup_pass(gpu, &mut pass, self.scene.active_camera.as_ref().unwrap());
  }
}
