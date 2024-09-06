use crate::*;

struct UniformArrayLights<T: Std140, const N: usize>(
  pub UniformBufferDataView<Shader140Array<T, N>>,
);

impl<T, const N: usize> ShaderPassBuilder for UniformArrayLights<T, N>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.0);
  }
}
impl<T, const N: usize> ShaderHashProvider for UniformArrayLights<T, N>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    todo!()
  }
}

impl<T, const N: usize> LightingComputeComponent for UniformArrayLights<T, N>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation> {
    todo!()
  }
}

struct UniformArrayLightsInvocation<T: Std140, const N: usize>(
  pub UniformNode<Shader140Array<T, N>>,
);

impl<T: Std140, const N: usize> LightingComputeInvocation for UniformArrayLightsInvocation<T, N> {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    todo!()
  }
}
