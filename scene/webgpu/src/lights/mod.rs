pub mod directional;
pub use directional::*;

use crate::*;

#[derive(Default)]
pub struct LightList<T> {
  pub lights: Vec<T>,
}

impl<T> LightList<T> {
  //
}

pub struct DrawDefer<'a, T, D, S, R> {
  pub light: &'a T,
  pub defer: &'a D,
  pub shading: &'a S,
  pub target: &'a R,
}

pub trait DeferGBufferSchema<S: LightableSurfaceShading> {
  fn reconstruct_geometry_ctx(
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> ExpandedNode<ShaderLightingGeometricCtx>;

  fn reconstruct_shading(builder: &mut ShaderGraphFragmentBuilder) -> ExpandedNode<S>;
}

pub trait LightPassSchema {
  fn write_lighting(
    builder: &mut ShaderGraphFragmentBuilder,
    result: ExpandedNode<ShaderLightingResult>,
  );
}

/// super clean, super powerful
impl<'a, T, S, D, R> ShaderGraphProvider for DrawDefer<'a, T, D, S, R>
where
  T: DirectShaderLight,
  S: LightableSurfaceShading,
  D: DeferGBufferSchema<S>,
  R: LightPassSchema,
{
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let geom_ctx = D::reconstruct_geometry_ctx(builder);

      let shading = D::reconstruct_shading(builder);

      let incident_light = T::compute_direct_light(todo!(), &geom_ctx);

      let result = S::compute_lighting(&shading, &incident_light, &geom_ctx);

      R::write_lighting(builder, result);

      Ok(())
    })
  }
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
