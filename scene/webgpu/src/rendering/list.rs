use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) models: Vec<Box<dyn SceneRenderable>>,
}

impl RenderList {
  pub fn setup_pass<P: SceneContent>(
    &self,
    gpu_pass: &mut SceneRenderPass,
    scene: &mut Scene<P>,
    dispatcher: &dyn RenderComponentAny,
  ) {
    self
      .models
      .iter()
      .for_each(|model| model.render(gpu_pass, dispatcher, scene.active_camera.as_ref().unwrap()))
  }
}
