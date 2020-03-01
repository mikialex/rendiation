use crate::geometry::StandardGeometry;
use crate::renderer::pipeline::WGPUPipeline;
use crate::renderer::WGPURenderer;
use crate::{
  geometry_lib::{sphere_geometry::SphereGeometryParameter, Mesher},
  renderer::render_pass::WGPURenderPass,
  StaticPipelineBuilder,
};
use rendiation_render_entity::Camera;

pub struct Scene {
  background: Box<dyn Background>,
  cameras: Vec<Box<dyn Camera>>,
  geometries: Vec<StandardGeometry>,
  // nodes: Vec<SceneNode>
}

impl Scene {
  // pub fn
}

// pub trait SceneNode{
// }

pub trait Renderable {
  fn prepare(&mut self, renderer: &mut WGPURenderer);
  fn render(&self, pass: &WGPURenderPass);
}

// pub struct RenderObject {
//     geometry: StandardGeometry,
//     shading:
// }

pub trait Background: Renderable {}

pub struct Sky {
  geometry: StandardGeometry,
  pipeline: WGPUPipeline,
}

impl Sky {
  pub fn new(renderer: &mut WGPURenderer) -> Self {
    let mut geometry: StandardGeometry = SphereGeometryParameter::default().create_mesh().into();
    geometry.update_gpu(renderer);
    todo!()
    // let mut builder = StaticPipelineBuilder::new(
    //   renderer,
    //   include_str!(),
    //   include_str!(),
    // );
    // Sky { geometry }
  }
}
