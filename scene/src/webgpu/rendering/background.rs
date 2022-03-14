use rendiation_webgpu::*;

use crate::*;

pub struct BackGroundRendering;

impl PassContentWithSceneAndCamera for BackGroundRendering {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass, scene: &Scene, camera: &SceneCamera) {
    scene
      .background
      .render(gpu, pass, &DefaultPassDispatcher, camera);
  }
}
