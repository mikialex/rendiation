use super::*;

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
pub struct ForwardScene {
  render_list: RenderList,
}

impl PassContent for ForwardScene {
  fn update(
    &mut self,
    gpu: &GPU,
    scene: &mut Scene,
    _resource: &mut ResourcePoolImpl,
    pass: &RenderPassInfo,
  ) {
    self.render_list.models.clear();

    scene.models.iter_mut().for_each(|model| {
      self.render_list.models.push(model.clone());
    });

    self.render_list.update(scene, gpu, pass);
  }

  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, scene: &'a Scene) {
    self.render_list.setup_pass(pass, scene);
  }
}
