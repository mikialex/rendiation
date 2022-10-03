pub mod directional;
pub use directional::*;
pub mod point;
pub use point::*;
pub mod spot;
pub use spot::*;

use crate::*;

pub trait WebGPUSceneLight: Any {
  fn collect(&self, res: &mut ForwardLightingSystem, node: &SceneNode);
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  /// from light source to surface
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
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized + Default
{
  /// this is to avoid mutable borrow errors in for_by and if_by.
  type Dependency;
  fn create_dep(builder: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency;
  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
    shading: &dyn Any,
  ) -> ExpandedNode<ShaderLightingResult>;
}

pub trait PunctualShaderLight:
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized + Default
{
  type Dependency;
  fn create_dep(builder: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency;
  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    dep: &Self::Dependency,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}

/// Punctual lights are defined as parameterized, infinitely small points that emit light in
/// well-defined directions and intensities.
pub trait PunctualLight:
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized + Default
{
  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}
