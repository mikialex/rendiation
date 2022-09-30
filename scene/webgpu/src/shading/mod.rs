pub mod physical;
pub use physical::*;

use crate::*;

pub trait LightableSurfaceShading: Any {
  type ShaderStruct: ShaderGraphStructuralNodeType;
  /// define how we construct a shader material instance from shader build ctx
  fn construct_shading(
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> ExpandedNode<Self::ShaderStruct>;

  /// define how we compute result lighting from a give pixel of surface and lighting
  fn compute_lighting(
    self_node: &ExpandedNode<Self::ShaderStruct>,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult>;
}

pub trait LightableSurfaceShadingDyn: Any {
  fn construct_shading_dyn(&self, builder: &mut ShaderGraphFragmentBuilder) -> Box<dyn Any>;

  fn compute_lighting_dyn(
    &self,
    self_node: &dyn Any,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult>;
}
impl<T: LightableSurfaceShading> LightableSurfaceShadingDyn for T {
  fn construct_shading_dyn(&self, builder: &mut ShaderGraphFragmentBuilder) -> Box<dyn Any> {
    Box::new(Self::construct_shading(builder))
  }

  fn compute_lighting_dyn(
    &self,
    self_node: &dyn Any,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult> {
    let self_node = self_node
      .downcast_ref::<ExpandedNode<<Self as LightableSurfaceShading>::ShaderStruct>>()
      .unwrap();
    Self::compute_lighting(self_node, direct_light, ctx)
  }
}
