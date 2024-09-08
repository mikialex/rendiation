use crate::*;

pub struct UniformArrayLights<T: Std140, const N: usize, U>(
  pub UniformBufferDataView<Shader140Array<T, N>>,
  pub PhantomData<U>,
  pub Arc<dyn Fn(Node<T>) -> U>,
);

/// should we consider impl such trait for containers in upstream?
impl<T, const N: usize, U> ShaderPassBuilder for UniformArrayLights<T, N, U>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.0);
  }
}
impl<T, const N: usize, U: 'static> ShaderHashProvider for UniformArrayLights<T, N, U>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  shader_hash_type_id! {}
}

impl<T, const N: usize, U> LightingComputeComponent for UniformArrayLights<T, N, U>
where
  T: Std140 + ShaderSizedValueNodeType,
  U: LightingComputeInvocation + ShaderAbstractRightValue + Default,
{
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightingComputeInvocation> {
    let node = binding.bind_by(&self.0);
    let f = self.2.clone();
    Box::new(IterAsLightInvocation(node.into_shader_iter().map(
      move |(_, light): (Node<u32>, UniformNode<T>)| f(light.load()),
    )))
  }
}
