use rendiation_webgpu::{GPURenderPass, GPU};

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<MeshModel>,
}

impl RenderList {
  pub fn update(&mut self, scene: &mut Scene, gpu: &GPU, pass: &PassTargetFormatInfo) {
    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) =
        active_camera.get_updated_gpu(gpu, &scene.components.nodes.borrow());

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass,
        resources: &mut scene.resources,
      };

      self.models.iter_mut().for_each(|model| {
        let components = &mut scene.components;
        model.update(gpu, &mut base, components);
      });
    }
  }

  pub fn setup_pass<'p>(
    &self,
    gpu_pass: &mut GPURenderPass<'p>,
    scene: &'p Scene,
    pass: &'p PassTargetFormatInfo,
  ) {
    self.models.iter().for_each(|model| {
      model.setup_pass(
        gpu_pass,
        &scene.components,
        scene.active_camera.as_ref().unwrap().expect_gpu(),
        &scene.resources,
        pass,
      )
    })
  }
}
