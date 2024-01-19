mod physical;
pub use physical::*;

use crate::*;

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  /// from light source to surface
  pub direction: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

// note, we have to use the real name but not the ENode<ShaderAPIInstance> or we can not pass the
// rust orphan rules
impl core::ops::Add for ShaderLightingResultShaderAPIInstance {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      diffuse: self.diffuse + rhs.diffuse,
      specular: self.specular + rhs.specular,
    }
  }
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingGeometricCtx {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  /// from surface to the camera
  pub view_dir: Vec3<f32>,
}

pub trait LightableSurfaceShading: std::any::Any {
  type ShaderStruct: ShaderStructuralNodeType;
  /// define how we construct a shader material instance from shader build ctx
  fn construct_shading(builder: &mut ShaderFragmentBuilder) -> ENode<Self::ShaderStruct>;

  /// define how we compute result lighting from a give pixel of surface and lighting
  fn compute_lighting_by_incident(
    self_node: &ENode<Self::ShaderStruct>,
    incident: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult>;
}

pub trait LightableSurfaceShadingDyn: Any {
  fn construct_shading_dyn(&self, builder: &mut ShaderFragmentBuilder) -> Box<dyn Any>;

  fn compute_lighting_by_incident_dyn(
    &self,
    self_node: &dyn Any,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult>;
}
impl<T: LightableSurfaceShading> LightableSurfaceShadingDyn for T {
  fn construct_shading_dyn(&self, builder: &mut ShaderFragmentBuilder) -> Box<dyn Any> {
    Box::new(Self::construct_shading(builder))
  }

  fn compute_lighting_by_incident_dyn(
    &self,
    self_node: &dyn Any,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let self_node = self_node
      .downcast_ref::<ENode<<Self as LightableSurfaceShading>::ShaderStruct>>()
      .unwrap();
    Self::compute_lighting_by_incident(self_node, direct_light, ctx)
  }
}
