use rendiation_lighting_transport::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

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
