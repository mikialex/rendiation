use crate::*;

pub struct BackGroundRendering;

impl PassContentWithSceneAndCamera for BackGroundRendering {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    if let Some(bg) = &scene.scene.background {
      match bg {
        SceneBackGround::Solid(bg) => bg.render(pass, &default_dispatcher(pass), camera, scene),
        SceneBackGround::Env(bg) => bg.render(pass, &default_dispatcher(pass), camera, scene),
        SceneBackGround::Foreign(_) => {}
      }
    }
  }
}
