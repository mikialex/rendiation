use rendiation_webgpu::{RenderPassInfo, GPU};

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<MeshModel>,
}

impl RenderList {
  pub fn update(&mut self, scene: &mut Scene, gpu: &GPU, pass: &RenderPassInfo) {
    let mut base = scene.create_material_ctx_base(gpu, pass, &DefaultPassDispatcher);

    self.models.iter_mut().for_each(|model| {
      model.update(gpu, &mut base);
    });
  }

  pub fn setup_pass<'p>(&self, gpu_pass: &mut SceneRenderPass<'p>, scene: &'p Scene) {
    self.models.iter().for_each(|model| {
      model.setup_pass(
        gpu_pass,
        scene
          .resources
          .cameras
          .expect_gpu(scene.active_camera.as_ref().unwrap()),
        &scene.resources,
      )
    })
  }
}
