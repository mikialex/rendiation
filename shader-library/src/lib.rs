
pub mod tone_mapping;

#[test]
fn test(){
  // let a = uncharted2ToneMappingFunction::name();
  // println!("{}", uncharted2ToneMappingFunction::name());
}

use rendiation_shadergraph_derives::{UniformBuffer, BindGroup};
use rendiation_math::Mat4;
use rendiation_shadergraph::ShaderGraphBindGroupItemProvider;

#[derive(UniformBuffer)]
pub struct MVPTransformed {
  pub projection: Mat4<f32>,
  pub model_view: Mat4<f32>,
}

// struct MVPTransformedShaderGraphInstance {
//   projection: ShaderGraphNodeHandle<Mat4<f32>>,
//   model_view: ShaderGraphNodeHandle<Mat4<f32>>,
// }

// impl ShaderGraphUniformBufferProvider for MVPTransformed{
//   type ShaderGraphUniformBufferInstance = MVPTransformedShaderGraphInstance
//   fn create_instance<'a>(bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>) -> Self::ShaderGraphUniformBufferInstance {
//     Self{
//       projection: bindgroup_builder.uniform::<Mat4<f32>>("projection"),
//       model_view:bindgroup_builder.uniform::<Mat4<f32>>("model_view"),
//     }
//   }
// }

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  // #[bind_stage = "vertex"]
  pub uniforms: MVPTransformed,

  // #[bind_stage = "fragment"]
  // pub texture_view: TextureView, // need cal stuff?

  // #[bind_stage = "fragment"]
  // pub sampler: Sampler,

}

impl BlockShadingParamGroup{
  pub fn create_layout(){

  }

  pub fn create_bindgroup(
    // .. many params
  ){

  }
}
