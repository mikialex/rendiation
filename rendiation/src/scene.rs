use crate::geometry::StandardGeometry;
use crate::renderer::pipeline::WGPUPipeline;
use crate::renderer::WGPURenderer;
use crate::{
  geometry_lib::{sphere_geometry::SphereGeometryParameter, Mesher},
  renderer::render_pass::WGPURenderPass,
};

pub struct Scene {
  geometries: Vec<StandardGeometry>,
}

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
  //   pipeline: WGPUPipeline,
}

impl Sky {
  pub fn new(renderer: &mut WGPURenderer) -> Self {
    let mut geometry: StandardGeometry = SphereGeometryParameter::default().create_mesh().into();
    geometry.update_gpu(renderer);

    // let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    // pipeline_builder
    //   .vertex_shader(include_str!("./block.vert"))
    //   .frag_shader(include_str!("./block.frag"))
    //   .binding_group(
    //     BindGroupLayoutBuilder::new()
    //       .bind_uniform_buffer(ShaderStage::Vertex)
    //       .bind_texture2d(ShaderStage::Fragment)
    //       .bind_sampler(ShaderStage::Fragment)
    //   )
    //   .to_screen_target(&renderer)

    Sky { geometry }
  }
}
