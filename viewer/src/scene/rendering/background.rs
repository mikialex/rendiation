use rendiation_webgpu::GPU;

use crate::*;

#[derive(Default)]
pub struct BackGroundRendering;

impl PassContent for BackGroundRendering {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) = active_camera.get_updated_gpu(gpu);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass_info: ctx.pass_info,
        resources: &mut scene.resources,
        pass: &DefaultPassDispatcher,
      };

      scene.background.update(gpu, &mut base);
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    scene.background.setup_pass(
      pass,
      scene.active_camera.as_ref().unwrap().expect_gpu(),
      &scene.resources,
    );
  }
}
