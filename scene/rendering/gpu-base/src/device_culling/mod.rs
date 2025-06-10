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

pub fn use_scene_model_device_world_bounding(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<SceneDrawUnitWorldBoundingProviderDefaultImpl> {
  qcx
    .use_storage_buffer(|gpu| {
      let source = scene_model_world_bounding()
        .collective_map(|b| [b.min, b.max])
        .into_query_update_storage(0);
      create_reactive_storage_buffer_container::<[f32; 6]>(128, u32::MAX, gpu).with_source(source)
    })
    .map(|bounding_storage| SceneDrawUnitWorldBoundingProviderDefaultImpl { bounding_storage })
}

#[derive(Clone)]
pub struct SceneDrawUnitWorldBoundingProviderDefaultImpl {
  bounding_storage: StorageBufferReadonlyDataView<[[f32; 6]]>,
}

impl ShaderHashProvider for SceneDrawUnitWorldBoundingProviderDefaultImpl {
  shader_hash_type_id! {}
}
impl DrawUnitWorldBoundingProvider for SceneDrawUnitWorldBoundingProviderDefaultImpl {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn DrawUnitWorldBoundingInvocationProvider> {
    Box::new(SceneDrawUnitWorldBoundingInvocationProviderDefaultImpl {
      bounding_storage: cx.bind_by(&self.bounding_storage),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.bounding_storage);
  }
}

struct SceneDrawUnitWorldBoundingInvocationProviderDefaultImpl {
  bounding_storage: ShaderReadonlyPtrOf<[[f32; 6]]>,
}

impl DrawUnitWorldBoundingInvocationProvider
  for SceneDrawUnitWorldBoundingInvocationProviderDefaultImpl
{
  fn get_world_bounding(&self, id: Node<u32>) -> TargetWorldBounding {
    let b = self.bounding_storage.index(id).load();
    TargetWorldBounding {
      min: (b.index(0), b.index(1), b.index(2)).into(),
      max: (b.index(0), b.index(1), b.index(2)).into(),
    }
  }
}
