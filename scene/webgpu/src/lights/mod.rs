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
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    dep: &Self::Dependency,
  ) -> ExpandedNode<ShaderLightingResult>;
}

/// Punctual lights are defined as parameterized, infinitely small points that
/// emit light in well-defined directions and intensities.
pub trait PunctualShaderLight:
  ShaderGraphStructuralNodeType + ShaderStructMemberValueNodeType + Std140 + Sized + Default
{
  type PunctualDependency;
  fn create_punctual_dep(builder: &mut ShaderGraphFragmentBuilderView) -> Self::PunctualDependency;
  fn compute_incident_light(
    light: &ExpandedNode<Self>,
    dep: &Self::PunctualDependency,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}

impl<T: PunctualShaderLight> ShaderLight for T {
  type Dependency = T::PunctualDependency;

  fn create_dep(builder: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency {
    T::create_punctual_dep(builder)
  }

  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    dep: &Self::Dependency,
  ) -> ExpandedNode<ShaderLightingResult> {
    let incident = T::compute_incident_light(light, dep, ctx);
    shading_impl.compute_lighting_by_incident_dyn(shading, &incident, ctx)
  }
}
