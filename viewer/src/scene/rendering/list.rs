use rendiation_webgpu::{GPURenderPass, GPU};

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<ModelHandle>,
}

impl RenderList {
  pub fn update(&mut self, scene: &mut Scene, gpu: &GPU, pass: &PassTargetFormatInfo) {
    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) = active_camera.get_updated_gpu(gpu, &scene.components.nodes);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass,
        resources: &mut scene.resources,
      };

      let models = &scene.models;
      self.models.iter().for_each(|handle| {
        let model = models.get(*handle).unwrap();
        let components = &mut scene.components;
        let material = components
          .materials
          .get_mut(model.material())
          .unwrap()
          .as_mut();
        let mesh = components.meshes.get_mut(model.mesh()).unwrap();
        let node = components.nodes.get_node_mut(model.node()).data_mut();

        let mut ctx = SceneMaterialRenderPrepareCtx {
          base: &mut base,
          model_info: node.get_model_gpu(gpu).into(),
          active_mesh: mesh.as_ref().into(),
        };

        material.update(gpu, &mut ctx);

        mesh.update(gpu, &mut base.resources.custom_storage);
      });
    }
  }

  pub fn setup_pass<'p>(
    &self,
    gpu_pass: &mut GPURenderPass<'p>,
    scene: &'p Scene,
    pass: &'p PassTargetFormatInfo,
  ) {
    let models = &scene.models;

    self.models.iter().for_each(|model| {
      let model = models.get(*model).unwrap();
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
