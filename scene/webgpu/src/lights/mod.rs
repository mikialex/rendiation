pub mod directional;
pub use directional::*;
use rendiation_algebra::*;
use shadergraph::*;

pub struct LightList<T> {
  pub lights: Vec<T>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderIncidentLight {
  pub color: Vec3<f32>,
  pub direction: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingResult {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
}

#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLightingGeometricCtx {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub view_dir: Vec3<f32>,
}

pub trait ShaderLight: ShaderGraphStructuralNodeType + Sized {
  fn name() -> &'static str;
}

pub trait DirectShaderLight: ShaderLight {
  fn compute_direct_light(
    node: &ExpandedNode<Self>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight>;
}
