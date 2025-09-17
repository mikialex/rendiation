use crate::*;

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
  pub min: Node<HighPrecisionTranslation>,
  pub max: Node<HighPrecisionTranslation>,
}

pub fn use_scene_model_device_world_bounding(
  cx: &mut QueryGPUHookCx,
) -> Option<DrawUnitWorldBoundingProviderDefaultImpl> {
  let (cx, storage) = cx.use_storage_buffer("scene model world bounding", 128, u32::MAX);

  cx.use_shared_dual_query(SceneModelWorldBounding)
    .into_delta_change()
    .map_changes(|b| {
      let min = into_hpt(b.min);
      let max = into_hpt(b.max);
      [min.f1, min.f2, max.f1, max.f2]
    })
    .update_storage_array(cx, storage, 0);

  storage.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);
  storage.use_update(cx);

  cx.when_render(|| DrawUnitWorldBoundingProviderDefaultImpl {
    bounding_storage: storage.get_gpu_buffer(),
  })
}

#[derive(Clone)]
pub struct DrawUnitWorldBoundingProviderDefaultImpl {
  bounding_storage: AbstractReadonlyStorageBuffer<[[f32; 12]]>,
}

impl ShaderHashProvider for DrawUnitWorldBoundingProviderDefaultImpl {
  shader_hash_type_id! {}
}
impl DrawUnitWorldBoundingProvider for DrawUnitWorldBoundingProviderDefaultImpl {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn DrawUnitWorldBoundingInvocationProvider> {
    Box::new(DrawUnitWorldBoundingInvocationProviderDefaultImpl {
      bounding_storage: cx.bind_by(&self.bounding_storage),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.bounding_storage);
  }
}

struct DrawUnitWorldBoundingInvocationProviderDefaultImpl {
  bounding_storage: ShaderReadonlyPtrOf<[[f32; 12]]>,
}

impl DrawUnitWorldBoundingInvocationProvider
  for DrawUnitWorldBoundingInvocationProviderDefaultImpl
{
  fn get_world_bounding(&self, id: Node<u32>) -> TargetWorldBounding {
    let b = self.bounding_storage.index(id).load();
    TargetWorldBounding {
      min: ENode::<HighPrecisionTranslation> {
        f1: (b.index(0), b.index(1), b.index(2)).into(),
        f2: (b.index(3), b.index(4), b.index(5)).into(),
      }
      .construct(),
      max: ENode::<HighPrecisionTranslation> {
        f1: (b.index(6), b.index(7), b.index(8)).into(),
        f2: (b.index(9), b.index(10), b.index(11)).into(),
      }
      .construct(),
    }
  }
}
