use super::scene::{Renderable, Scene, ScenePrepareCtx};
use rendiation::*;
use rendiation_math::Vec3;

pub trait Background: Renderable {}

pub struct SolidBackground {
  pub color: Vec3<f32>,
}

impl SolidBackground {
  pub fn new() -> Self {
    Self {
      color: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}

impl Renderable for SolidBackground {
  fn prepare(&mut self, _: &mut WGPURenderer, _: &mut ScenePrepareCtx) {}
  fn render(&self, renderer: &WGPURenderer, _: &Scene) {

    // // just use a clear pass, todo, merge clear pass to follower pass
    // let mut pass = WGPURenderPass::build()
    //   .output_with_clear(target, (0.1, 0.2, 0.3, 1.0))
    //   .create(&mut renderer.encoder);
  }
}

impl Background for SolidBackground {}

pub struct Sky {
  geometry: StandardGeometry,
  pipeline: WGPUPipeline,
}

impl Sky {
  pub fn new(renderer: &mut WGPURenderer) -> Self {
    // let mut geometry: StandardGeometry = SphereGeometryParameter::default().create_mesh().into();
    // geometry.update_gpu(renderer);
    todo!()
    // let mut builder = StaticPipelineBuilder::new(
    //   renderer,
    //   include_str!(),
    //   include_str!(),
    // );
    // Sky { geometry }
  }
}
