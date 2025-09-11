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
    scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation> {
    Box::new(LightingComputeInvocationGroup {
      comps: self
        .comps
        .iter()
        .map(|c| c.build_light_compute_invocation(binding, scene_id))
        .collect(),
    })
  }
  fn setup_pass(&self, ctx: &mut BindingBuilder) {
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
    let light_specular_and_emissive_result = val(Vec3::<f32>::zero()).make_local_var();
    let light_diffuse_result = val(Vec3::<f32>::zero()).make_local_var();

    self.comps.iter().for_each(|light| {
      let r = light.compute_lights(shading, geom_ctx);
      light_specular_and_emissive_result
        .store(light_specular_and_emissive_result.load() + r.specular_and_emissive);
      light_diffuse_result.store(light_diffuse_result.load() + r.diffuse);
    });

    ENode::<ShaderLightingResult> {
      diffuse: light_diffuse_result.load(),
      specular_and_emissive: light_specular_and_emissive_result.load(),
    }
  }
}
