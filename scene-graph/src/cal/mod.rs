// Content abstaction layer

use rendiation::*;
use std::any::Any;
use rendiation_mesh_buffer::geometry::IndexedGeometry;

pub trait CALBackend {
  type Shading;
  fn create_shading(des: &SceneShadingDescriptor) -> Self::Shading;
  fn dispose_shading(shading: Self::Shading);

  type Uniform;
  fn create_uniform_buffer(des: SceneUniform) -> Self::Uniform;

//   type Geometry;
//   fn create_geometry(des: IndexedGeometry) -> Self::Geometry;
}

pub struct SceneShadingDescriptor {
  pub vertex_shader_str: String, // new sal(shading abstraction layer) is in design, assume shader just works
  pub frag_shader_str: String,
  // .. blend state stuff
}

pub struct SceneUniform{
    pub value: Box<dyn Any>
}

struct WebGPUCALBackend{}

impl CALBackend for WebGPUCALBackend{
    type Shading = WGPUPipeline;
    fn create_shading(_des: &SceneShadingDescriptor) -> Self::Shading{
        todo!()
    }
    fn dispose_shading(_shading: Self::Shading){
        // just drop!
    }
    type Uniform = WGPUBuffer;
    fn create_uniform_buffer(_des: SceneUniform) -> Self::Uniform{
        todo!()
    }
}