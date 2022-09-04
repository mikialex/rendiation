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
  fn compute_direct_light(
    builder: &mut ShaderGraphFragmentBuilderView,
    node: &ExpandedNode<Self>,
    _ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight> {
    //
    ExpandedNode::<ShaderIncidentLight> {
      color: node.intensity,
      direction: node.direction,
    }
  }
}
