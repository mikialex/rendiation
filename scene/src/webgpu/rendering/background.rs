use rendiation_webgpu::*;

use crate::*;

pub struct BackGroundRendering<'a> {
  scene: &'a Scene,
}

impl<'a> PassContentWithCamera for BackGroundRendering<'a> {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    self
      .scene
      .background
      .setup_pass(gpu, pass, &DefaultPassDispatcher, camera);
  }
}

impl Scene {
  pub fn render_background(&self) -> impl PassContent + '_ {
    self.render_by_main_camera(BackGroundRendering { scene: self })
  }
}
