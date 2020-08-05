
pub mod tone_mapping;

#[test]
fn test(){
  // let a = uncharted2ToneMappingFunction::name();
  // println!("{}", uncharted2ToneMappingFunction::name());
}

use rendiation_shadergraph_derives::UniformBuffer;
use rendiation_math::Mat4;

#[derive(UniformBuffer)]
pub struct MVPTransformed {
  pub projection: Mat4<f32>,
  pub model_view: Mat4<f32>,
}
