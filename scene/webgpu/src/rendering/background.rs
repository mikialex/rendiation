use crate::*;

pub struct BackGroundRendering;

impl<S> PassContentWithSceneAndCamera<S> for BackGroundRendering
where
  ,
  S::BackGround: Deref<Target = dyn WebGPUBackground>,
{
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera) {
    scene
      .background
      .as_ref()
      .unwrap()
      .render(pass, &pass.default_dispatcher(), camera);
  }
}
