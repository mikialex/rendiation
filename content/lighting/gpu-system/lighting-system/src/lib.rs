use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_lighting_transport::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod combine;
pub use combine::*;

mod uniform;
pub use uniform::*;

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

impl LightingComputeInvocation for Box<dyn PunctualShaderLight> {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let incident = self.compute_incident_light(geom_ctx);
    shading.compute_lighting_by_incident(&incident, geom_ctx)
  }
}

impl LightingComputeInvocation for (Box<dyn PunctualShaderLight>, Box<dyn ShadowOcclusionQuery>) {
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
