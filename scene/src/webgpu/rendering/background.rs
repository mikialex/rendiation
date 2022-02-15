use rendiation_webgpu::GPU;

use crate::*;

#[derive(Default)]
pub struct BackGroundRendering;

impl PassContent for BackGroundRendering {
  fn setup_pass<'a>(&self, gpu: &GPU, pass: &mut SceneRenderPass<'a>, scene: &Scene) {
    if let Some(camera) = &mut scene.active_camera {
      let camera = scene
        .resources
        .content
        .cameras
        .check_update_gpu(camera, gpu);
      scene
        .background
        .setup_pass(gpu, pass, camera, &mut scene.resources);
    }
  }
}
