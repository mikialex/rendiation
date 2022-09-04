use rendiation_algebra::Vec3;
use shadergraph::*;

use crate::{ShaderIncidentLight, ShaderLight, ShaderLightingGeometricCtx};

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
}

impl ShaderLight for DirectionalLightShaderInfo {
  fn name() -> &'static str {
    "directional_light"
  }
  type Dependency = ();
  fn create_dep(builder: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency {}
  fn compute_direct_light(
    node: &ExpandedNode<Self>,
    _dep: &Self::Dependency,
    _ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight> {
    //
    ExpandedNode::<ShaderIncidentLight> {
      color: node.intensity,
      direction: node.direction,
    }
  }
}
