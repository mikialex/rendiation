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

impl core::ops::Add for ENode<ShaderLightingResult> {
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
  /// origin from surface to the camera
  pub view_dir: Vec3<f32>,
}

pub trait LightableSurfaceShadingProvider {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilder,
  ) -> Box<dyn LightableSurfaceShading>;
}

pub trait LightableSurfaceShading {
  fn compute_lighting_by_incident(
    &self,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult>;
}
