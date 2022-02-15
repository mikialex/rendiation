use rendiation_webgpu::GPU;

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<Box<dyn SceneRenderable>>,
}

impl RenderList {
  pub fn setup_pass<'p>(&self, gpu: &GPU, gpu_pass: &mut SceneRenderPass<'p>, scene: &mut Scene) {
    self.models.iter().for_each(|model| {
      model.setup_pass(
        gpu,
        gpu_pass,
        scene.active_camera.as_ref().unwrap(),
        &mut scene.resources,
      )
    })
  }
}
