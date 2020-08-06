
pub mod tone_mapping;

#[test]
fn test(){
  // let a = uncharted2ToneMappingFunction::name();
  // println!("{}", uncharted2ToneMappingFunction::name());
}

use rendiation_shadergraph_derives::{UniformBuffer, BindGroup, Geometry};
use rendiation_math::{Vec3, Mat4, Vec2};
use rendiation_shadergraph::*;

#[derive(UniformBuffer)]
pub struct MVPTransformed {
  pub projection: Mat4<f32>,
  pub model_view: Mat4<f32>,
}

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  // #[bind_stage = "vertex"]
  pub uniforms: MVPTransformed,

  // #[bind_stage = "fragment"]
  // pub texture_view: TextureView, // need cal stuff?

  // #[bind_stage = "fragment"]
  pub sampler: ShaderGraphSampler,
}

#[repr(C)]
#[derive(Clone, Copy, Geometry)]
pub struct Vertex {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub uv: Vec2<f32>,
}


// this will be marco yes!
impl BlockShadingParamGroup{
  pub fn create_layout(){

  }

  pub fn create_bindgroup(
    // .. many params
  ){

  }
}


pub struct BlockShader {
  pub fog_type: bool,
}

impl BlockShader {
  pub fn create_shader(&self) -> ShaderGraph {
    let mut builder = ShaderGraphBuilder::new();
    let geometry = builder.geometry_by::<Vertex>();
    let block_paramter = builder.bindgroup_by::<BlockShadingParamGroup>();

    let uniforms = block_paramter.uniforms;
    let projection = uniforms.projection;
    let position = geometry.position;

    builder.create()
  }
}

// trait ShaderFactory {
//   type Builder;
//   fn create_shader(&self, builder: Builder) -> ShaderGraph;
// }
