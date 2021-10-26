use rendiation_webgpu::{GPURenderPass, GPU};

use crate::*;

pub struct BackGroundRendering;

impl PassContent for BackGroundRendering {
  fn update(
    &mut self,
    gpu: &GPU,
    scene: &mut Scene,
    _resource: &mut ResourcePoolInner,
    pass_info: &PassTargetFormatInfo,
  ) {
    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) =
        active_camera.get_updated_gpu(gpu, &scene.components.nodes.borrow());

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass: pass_info,
        resources: &mut scene.resources,
      };

      scene
        .background
        .update(gpu, &mut base, &mut scene.components);
    }
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut GPURenderPass<'a>,
    scene: &'a Scene,
    pass_info: &'a PassTargetFormatInfo,
  ) {
    scene.background.setup_pass(
      pass,
      &scene.components,
      scene.active_camera.as_ref().unwrap().expect_gpu(),
      &scene.resources,
      pass_info,
    );
  }
  //
}
