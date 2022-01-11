use rendiation_webgpu::GPU;

use crate::*;

impl Scene {
  pub fn get_main_pass_load_op(&self) -> wgpu::Operations<wgpu::Color> {
    let load = if let Some(clear_color) = self.background.require_pass_clear() {
      wgpu::LoadOp::Clear(clear_color)
    } else {
      wgpu::LoadOp::Load
    };

    wgpu::Operations { load, store: true }
  }
}

#[derive(Default)]
pub struct ForwardScene;

impl PassContent for ForwardScene {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    let (res, mut base) =
      scene.create_material_ctx_base(gpu, ctx.pass_info, &DefaultPassDispatcher);

    scene.models.iter_mut().for_each(|model| {
      model.update(gpu, &mut base, res);
    });
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    scene.models.iter().for_each(|model| {
      model.setup_pass(
        pass,
        scene
          .resources
          .content
          .cameras
          .expect_gpu(scene.active_camera.as_ref().unwrap()),
        &scene.resources,
      )
    })
  }
}
