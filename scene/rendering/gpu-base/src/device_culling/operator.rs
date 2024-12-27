use crate::*;

#[derive(Clone)]
pub struct NoopCuller;

impl ShaderHashProvider for NoopCuller {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for NoopCuller {
  fn create_invocation(&self, _: &mut ShaderBindGroupBuilder) -> Box<dyn AbstractCullerInvocation> {
    Box::new(NoopCullerInvocation)
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}

struct NoopCullerInvocation;
impl AbstractCullerInvocation for NoopCullerInvocation {
  fn cull(&self, _: Node<u32>) -> Node<bool> {
    val(false)
  }
}

#[derive(Clone)]
pub struct NotCuller(pub Box<dyn AbstractCullerProvider>);
impl ShaderHashProvider for NotCuller {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline_with_type_info(hasher);
  }
}

impl AbstractCullerProvider for NotCuller {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(NotCullerInvocation(self.0.create_invocation(cx)))
  }
  fn bind(&self, cx: &mut BindingBuilder) {
    self.0.bind(cx);
  }
}
struct NotCullerInvocation(Box<dyn AbstractCullerInvocation>);
impl AbstractCullerInvocation for NotCullerInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    self.0.cull(id).not()
  }
}

#[derive(Clone)]
pub struct ShortcutOrCuller(
  pub Box<dyn AbstractCullerProvider>,
  pub Box<dyn AbstractCullerProvider>,
);
impl ShaderHashProvider for ShortcutOrCuller {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline_with_type_info(hasher);
    self.1.hash_pipeline_with_type_info(hasher);
  }
}

impl AbstractCullerProvider for ShortcutOrCuller {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(ShortcutOrCullerInvocation(
      self.0.create_invocation(cx),
      self.1.create_invocation(cx),
    ))
  }
  fn bind(&self, cx: &mut BindingBuilder) {
    self.0.bind(cx);
  }
}
struct ShortcutOrCullerInvocation(
  Box<dyn AbstractCullerInvocation>,
  Box<dyn AbstractCullerInvocation>,
);
impl AbstractCullerInvocation for ShortcutOrCullerInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    let left = self.0.cull(id);
    let r = left.make_local_var();
    if_by(left.not(), || {
      r.store(r.load().or(self.1.cull(id)));
    });

    r.load()
  }
}
