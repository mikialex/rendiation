use rendiation_webgpu::GPU;

use crate::*;

impl Scene {
  // pub fn get_main_pass_load_op(&self) -> wgpu::Operations<wgpu::Color> {
  //   let load = if let Some(clear_color) = self.background.require_pass_clear() {
  //     wgpu::LoadOp::Clear(clear_color)
  //   } else {
  //     wgpu::LoadOp::Load
  //   };

  //   wgpu::Operations { load, store: true }
  // }
}

#[derive(Default)]
pub struct ForwardScene;

impl PassContent for ForwardScene {
  fn setup_pass<'a>(&self, gpu: &GPU, pass: &mut SceneRenderPass<'a>, scene: &mut Scene) {
    let resources = &mut scene.resources;
    scene.models.iter().for_each(|model| {
      model.setup_pass(gpu, pass, scene.active_camera.as_ref().unwrap(), resources)
    })
  }
}
