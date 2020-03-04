use crate::scene::Renderable;
use crate::renderer::WGPURenderer;
use crate::renderer::pipeline::WGPUPipeline;
use crate::geometry::StandardGeometry;

pub trait Background: Renderable {}

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
