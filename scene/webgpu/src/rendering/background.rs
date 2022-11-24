use crate::*;

pub struct BackGroundRendering;

impl PassContentWithSceneAndCamera for BackGroundRendering {
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene, camera: &SceneCamera) {
    if let Some(bg) = &scene.background {
      match bg {
        SceneBackGround::Solid(bg) => bg.render(pass, &pass.default_dispatcher(), camera),
        SceneBackGround::Env(bg) => bg.render(pass, &pass.default_dispatcher(), camera),
        SceneBackGround::Foreign(_) => todo!(),
        _ => {}
      }
    }
  }
}
