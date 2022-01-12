use rendiation_webgpu::GPU;

use crate::*;

#[derive(Default)]
pub struct BackGroundRendering;

impl PassContent for BackGroundRendering {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    if let Some(camera) = &mut scene.active_camera {
      scene
        .resources
        .content
        .cameras
        .check_update_gpu(camera, gpu);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        camera,
        pass_info: ctx.pass_info,
        resources: &mut scene.resources.content,
        pass: &DefaultPassDispatcher,
      };

      scene
        .background
        .update(gpu, &mut base, &mut scene.resources.scene);
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    scene.background.setup_pass(
      pass,
      scene
        .resources
        .content
        .cameras
        .expect_gpu(scene.active_camera.as_ref().unwrap()),
      &scene.resources,
    );
  }
}
