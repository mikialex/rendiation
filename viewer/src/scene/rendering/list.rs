use rendiation_webgpu::{GPURenderPass, RenderPassInfo, GPU};

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<MeshModel>,
}

impl RenderList {
  pub fn update(&mut self, scene: &mut Scene, gpu: &GPU, pass: &RenderPassInfo) {
    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) = active_camera.get_updated_gpu(gpu);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass,
        resources: &mut scene.resources,
      };

      self.models.iter_mut().for_each(|model| {
        model.update(gpu, &mut base);
      });
    }
  }

  pub fn setup_pass<'p>(&self, gpu_pass: &mut GPURenderPass<'p>, scene: &'p Scene) {
    self.models.iter().for_each(|model| {
      model.setup_pass(
        gpu_pass,
        scene.active_camera.as_ref().unwrap().expect_gpu(),
        &scene.resources,
      )
    })
  }
}
