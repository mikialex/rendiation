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
  C::ShaderBindResult: IntoShaderIterator<ShaderIter = S>,
  S: ShaderIterator<Item = T> + Clone + 'static,
  F: Fn(T) -> U + Copy + 'static,
  U: LightingComputeInvocation + ShaderAbstractRightValue + Default,
  T: Copy + 'static,
{
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation> {
    let node = self.0.bind_shader(binding);
    Box::new(IterAsLightInvocation(node.into_shader_iter().map(self.1)))
  }

  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.bind_pass(&mut ctx.binding);
  }
}

pub struct LightAndShadowCombined<L, S>(L, S);

impl<L: AbstractBindingSource, S: AbstractBindingSource> AbstractBindingSource
  for LightAndShadowCombined<L, S>
{
  type ShaderBindResult =
    LightAndShadowCombinedShaderInput<L::ShaderBindResult, S::ShaderBindResult>;

  fn bind_pass(&self, ctx: &mut BindingBuilder) {
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
