use crate::*;

#[derive(Default)]
pub struct LightingComputeComponentGroup {
  pub comps: Vec<Box<dyn LightingComputeComponent>>,
}

impl LightingComputeComponentGroup {
  pub fn with_light(mut self, comp: impl LightingComputeComponent + 'static) -> Self {
    self.comps.push(Box::new(comp));
    self
  }
}

impl ShaderHashProvider for LightingComputeComponentGroup {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self
      .comps
      .iter()
      .for_each(|c| c.hash_pipeline_with_type_info(hasher))
  }
}

impl LightingComputeComponent for LightingComputeComponentGroup {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(LightingComputeInvocationGroup {
      comps: self
        .comps
        .iter()
        .map(|c| c.build_light_compute_invocation(binding))
        .collect(),
    })
  }
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.comps.iter().for_each(|c| c.setup_pass(ctx))
  }
}

pub struct LightingComputeInvocationGroup {
  comps: Vec<Box<dyn LightingComputeInvocation>>,
}

impl LightingComputeInvocation for LightingComputeInvocationGroup {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let light_specular_result = val(Vec3::zero()).make_local_var();
    let light_diffuse_result = val(Vec3::zero()).make_local_var();

    self.comps.iter().for_each(|light| {
      light.compute_lights(shading, geom_ctx);
    });

    ENode::<ShaderLightingResult> {
      diffuse: light_diffuse_result.load(),
      specular: light_specular_result.load(),
    }
  }
}
