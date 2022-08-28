pub mod physical;
pub use physical::*;

use crate::*;

pub trait LightableSurfaceShading: ShaderGraphStructuralNodeType {
  /// define how we construct a shader material instance from shader build ctx
  fn construct_shading(builder: &mut ShaderGraphFragmentBuilder) -> ExpandedNode<Self>;

  /// define how we compute result lighting from a give pixel of surface and lighting
  fn compute_lighting(
    self_node: &ExpandedNode<Self>,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult>;
}
