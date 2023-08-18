mod forward;
pub use forward::*;
mod defer;
pub use defer::*;

use crate::*;

pub trait LightCollectionCompute: ShaderPassBuilder + ShaderHashProvider {
  fn compute_lights(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderBuildError>;

  fn compute_lights_grouped(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<ENode<ShaderLightingResult>, ShaderBuildError> {
    let (diffuse, specular) =
      self.compute_lights(builder, binding, shading_impl, shading, geom_ctx)?;
    Ok(ENode::<ShaderLightingResult> { diffuse, specular })
  }
}
