use crate::*;

pub struct ArrayLights<C, F>(pub C, pub F);

impl<C, F> ShaderHashProvider for ArrayLights<C, F>
where
  C: 'static,
  F: 'static,
{
  shader_hash_type_id! {}
}

impl<C, S, T, U, F> LightingComputeComponent for ArrayLights<C, F>
where
  C: AbstractBindingSource + 'static,
  C::ShaderBindResult: IntoShaderIterator<ShaderIter = S> + Clone,
  S: ShaderIterator<Item = T> + 'static,
  F: Fn(T) -> U + Copy + 'static,
  U: LightingComputeInvocation,
  T: Clone + 'static,
{
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation> {
    let node = self.0.bind_shader(binding);
    let light = node.map(self.1);
    Box::new(ShaderIntoIterAsLightInvocation(light))
  }

  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.bind_pass(ctx);
  }
}

pub struct ShaderIntoIterAsLightInvocation<T>(pub T);
impl<T> LightingComputeInvocation for ShaderIntoIterAsLightInvocation<T>
where
  T: IntoShaderIterator + Clone,
  <T::ShaderIter as ShaderIterator>::Item: LightingComputeInvocation,
{
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let light_specular_and_emissive_result = val(Vec3::<f32>::zero()).make_local_var();
    let light_diffuse_result = val(Vec3::<f32>::zero()).make_local_var();

    self.0.clone().into_shader_iter().for_each(|light, _| {
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

pub struct LightAndShadowCombinedSource<L, S>(pub L, pub S);

impl<L: AbstractBindingSource, S: AbstractBindingSource> AbstractBindingSource
  for LightAndShadowCombinedSource<L, S>
{
  type ShaderBindResult =
    LightAndShadowCombinedShaderInput<L::ShaderBindResult, S::ShaderBindResult>;

  fn bind_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.bind_pass(ctx);
    self.1.bind_pass(ctx);
  }

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    LightAndShadowCombinedShaderInput(self.0.bind_shader(ctx), self.1.bind_shader(ctx))
  }
}

#[derive(Clone, Copy)]
pub struct LightAndShadowCombinedShaderInput<L, S>(L, S);

impl<L: IntoShaderIterator, S: IntoShaderIterator> IntoShaderIterator
  for LightAndShadowCombinedShaderInput<L, S>
{
  type ShaderIter = ShaderZipIter<L::ShaderIter, S::ShaderIter>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    self.0.into_shader_iter().zip(self.1.into_shader_iter())
  }
}
