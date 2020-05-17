use super::scene::Renderable;
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
  fn render(&self, renderer: &mut WGPURenderer, builder: WGPURenderPassBuilder) {
    builder
      .first_color(|c| c.load_with_clear(self.color, 1.0).ok())
      .create(&mut renderer.encoder);
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
