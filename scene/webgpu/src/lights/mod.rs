pub mod directional;
pub use directional::*;

use crate::*;

pub struct LightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: UniformBufferDataView<[T; 32]>,
}

impl<T: ShaderLight> LightList<T> {
  pub fn update(&mut self) {
    //
  }

  pub fn collect_lights_for_naive_forward<S: LightableSurfaceShading>(
    builder: &mut ShaderGraphRenderPipelineBuilder,
    shading: ExpandedNode<S>,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let lights = todo!();
      let light_result = todo!();
      // for_by(lights, |_, _| {
      //   //
      // });
      Ok(())
    })
  }
}

pub trait LightCollection {
  fn collect_lights_for_naive_forward(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError>;
  fn draw_defer_passes(&self, ctx: &mut FrameCtx);
}

pub struct LightSystem {
  pub lights: Vec<Box<dyn LightCollection>>,
}

impl LightSystem {
  pub fn collect_lights_for_naive_forward<S: LightableSurfaceShading>(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for light in &self.lights {
      light.collect_lights_for_naive_forward(builder)?
    }
    Ok(())
  }

  pub fn draw_defer_passes(&self, ctx: &mut FrameCtx) {
    for light in &self.lights {
      light.draw_defer_passes(ctx);
    }
  }
}

pub struct DrawDefer<'a, T: 'static, D, S, R> {
  pub light: &'a UniformBufferDataView<T>,
  pub defer: &'a D,
  pub shading: &'a S,
  pub target: &'a R,
}

/// define a specific g buffer layout.
///
/// this trait is parameterized over shading, which means we could encode/reconstruct
/// different surface shading into one schema theoretically
pub trait DeferGBufferSchema<S: LightableSurfaceShading> {
  fn reconstruct_geometry_ctx(
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> ExpandedNode<ShaderLightingGeometricCtx>;

  fn reconstruct_shading(builder: &mut ShaderGraphFragmentBuilder) -> ExpandedNode<S>;
}

/// define a specific light buffer layout.
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
      let light = binding.uniform_by(self.light, SB::Pass).expand();

      let geom_ctx = D::reconstruct_geometry_ctx(builder);

      let shading = D::reconstruct_shading(builder);

      let incident_light = T::compute_direct_light(&light, &geom_ctx);

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
