use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_lighting_transport::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod combine;
pub use combine::*;

mod array;
pub use array::*;

pub trait LightingComputeComponent: ShaderHashProvider + ShaderPassBuilder {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation>;
}

pub trait LightingComputeInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult>;
}

impl<T> LightingComputeInvocation for Node<T>
where
  Node<T>: PunctualShaderLight,
{
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let incident = self.compute_incident_light(geom_ctx);
    shading.compute_lighting_by_incident(&incident, geom_ctx)
  }
}

impl<L, S> LightingComputeInvocation for (Node<L>, Node<S>)
where
  Node<L>: PunctualShaderLight,
  Node<S>: ShadowOcclusionQuery,
{
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let (light, shadow) = &self;
    let mut incident = light.compute_incident_light(geom_ctx);

    let occlusion = val(1.).make_local_var();
    if_by(incident.color.greater_than(Vec3::splat(0.)).all(), || {
      occlusion.store(shadow.query_shadow_occlusion(geom_ctx.position, geom_ctx.normal));
    });
    incident.color = incident.color * occlusion.load();

    shading.compute_lighting_by_incident(&incident, geom_ctx)
  }
}

pub struct IterAsLightInvocation<T>(pub T);
impl<T> LightingComputeInvocation for IterAsLightInvocation<T>
where
  T::Item: LightingComputeInvocation,
  T: ShaderIterator + Clone,
{
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let light_specular_result = val(Vec3::zero()).make_local_var();
    let light_diffuse_result = val(Vec3::zero()).make_local_var();

    self.0.clone().for_each(|light, _| {
      light.compute_lights(shading, geom_ctx);
    });

    ENode::<ShaderLightingResult> {
      diffuse: light_diffuse_result.load(),
      specular: light_specular_result.load(),
    }
  }
}
