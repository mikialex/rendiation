use crate::*;

pub struct BackGroundRendering;

impl PassContentWithSceneAndCamera for BackGroundRendering {
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene, camera: &SceneCamera) {
    scene
      .background
      .as_ref()
      .unwrap()
      .render(pass, &pass.default_dispatcher(), camera);
  }
}
