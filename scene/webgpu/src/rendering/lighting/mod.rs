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
  ) -> (Node<Vec3<f32>>, Node<Vec3<f32>>);

  fn compute_lights_grouped(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let (diffuse, specular) =
      self.compute_lights(builder, binding, shading_impl, shading, geom_ctx);
    ENode::<ShaderLightingResult> { diffuse, specular }
  }
}
