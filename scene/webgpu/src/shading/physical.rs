use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct PhysicalShading {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub roughness: f32,
}

impl LightableSurfaceShading for PhysicalShading {
  fn construct(builder: &ShaderGraphFragmentBuilder) -> ExpandedNode<Self> {
    todo!()
  }

  fn compute_lighting(
    self_node: &ExpandedNode<Self>,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult> {
    todo!()
  }
}
