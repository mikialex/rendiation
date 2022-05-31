use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<Box<dyn SceneRenderable>>,
}

impl RenderList {
  pub fn setup_pass<'p, 'a, 'r>(
    &self,
    gpu_pass: &mut SceneRenderPass<'p, 'a, 'r>,
    scene: &mut Scene<WebGPUScene>,
    dispatcher: &dyn RenderComponentAny,
  ) {
    self
      .models
      .iter()
      .for_each(|model| model.render(gpu_pass, dispatcher, scene.active_camera.as_ref().unwrap()))
  }
}
