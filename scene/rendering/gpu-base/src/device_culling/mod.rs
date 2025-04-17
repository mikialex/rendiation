use crate::*;

mod operator;
pub use operator::*;

mod culling;
pub use culling::*;

pub trait AbstractCullerProvider: ShaderHashProvider + DynClone {
  fn create_invocation(&self, cx: &mut ShaderBindGroupBuilder)
    -> Box<dyn AbstractCullerInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}

pub trait AbstractCullerInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool>;
}

impl ShaderHashProvider for Box<dyn AbstractCullerProvider> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }

  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    self.hash_type_info(hasher);
    self.hash_pipeline(hasher);
  }
}

impl AbstractCullerProvider for Box<dyn AbstractCullerProvider> {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    self.as_ref().create_invocation(cx)
  }
  fn bind(&self, cx: &mut BindingBuilder) {
    self.as_ref().bind(cx)
  }
}

pub trait AbstractCullerProviderExt: AbstractCullerProvider + Clone + 'static {
  fn not(&self) -> Box<dyn AbstractCullerProvider> {
    Box::new(NotCuller(Box::new(self.clone())))
  }

  fn shortcut_or(&self, other: Box<dyn AbstractCullerProvider>) -> Box<dyn AbstractCullerProvider> {
    Box::new(ShortcutOrCuller(Box::new(self.clone()), other))
  }
}
impl<T> AbstractCullerProviderExt for T where T: AbstractCullerProvider + Clone + 'static {}
dyn_clone::clone_trait_object!(AbstractCullerProvider);

pub trait DrawUnitWorldBoundingProvider: ShaderHashProvider + DynClone {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn DrawUnitWorldBoundingInvocationProvider>;
  fn bind(&self, cx: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(DrawUnitWorldBoundingProvider);

pub trait DrawUnitWorldBoundingInvocationProvider {
  fn get_world_bounding(&self, id: Node<u32>) -> TargetWorldBounding;
  fn should_not_as_occluder(&self, _id: Node<u32>) -> Node<bool> {
    val(false)
  }
}

pub struct TargetWorldBounding {
  pub min: Node<Vec3<f32>>,
  pub max: Node<Vec3<f32>>,
}
