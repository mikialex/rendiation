pub mod directional;
pub use directional::*;

use crate::*;

pub trait WebGPUSceneLight: Any {
  fn collect(&self, res: &mut ForwardLightingSystem, node: &SceneNode);
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  /// from source to target
  pub direction: Vec3<f32>,
}

only_fragment!(HDRLightResult, Vec3<f32>);
only_fragment!(LDRLightResult, Vec3<f32>);

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingGeometricCtx {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  /// from surface to the camera
  pub view_dir: Vec3<f32>,
}

pub trait ShaderLight:
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized
{
  type Dependency;
  fn name() -> &'static str;
  fn create_dep(builder: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency;
  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    dep: &Self::Dependency,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}
