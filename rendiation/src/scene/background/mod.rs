use crate::renderer::pipeline::WGPUPipeline;
use crate::renderer::WGPURenderer;

use super::scene::{Renderable, Scene, ScenePrepareCtx};
use crate::{
  geometry::StandardGeometry,
  geometry_lib::{sphere_geometry::SphereGeometryParameter, IndexedBufferMesher},
  WGPURenderPass,
};
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
  fn render(&self, _: &WGPURenderer, scene: &Scene) {
    // WGPURenderPass::build().output_with_clear(
    //   &scene.canvas.view(),
    //   (self.color.x, self.color.y, self.color.z, 1.0),
    // );
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
