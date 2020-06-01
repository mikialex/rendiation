pub mod manager;
pub mod shading;
pub mod geometry;
pub mod spg;
pub mod uniform;

pub use manager::*;
pub use shading::*;
pub use geometry::*;
pub use spg::*;
pub use uniform::*;

// /// webgpu => bindgroup
// /// webgl => nothing
// struct SceneShadingParameterGroup{
//   index: Index,
//   items: Vec<(Index, ShadingParameterType)>,
//   handle_index: Index,
// }

// enum ShadingParameterType{
//   UniformBuffer,
//   Texture,
//   Sampler,
// }

// /// webgpu => buffer
// /// webgl => uniform / ubo
// struct UniformBuffer{

// }

// impl SceneGraph{
//   pub fn create_shading(&mut self, shading: &SceneShadingDescriptor) -> &mut SceneShading{

//   }

//   pub fn create_shading_parameter_group() -> &mut SceneShadingParameterGroup{

//   }


// }