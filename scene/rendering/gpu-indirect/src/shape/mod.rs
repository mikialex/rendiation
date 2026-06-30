use crate::*;

mod attribute;
pub use attribute::*;

pub trait IndirectModelShapeRenderImpl:
  IndirectDrawProviderCreator + DrawCommandBuilderCreator
{
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  /// If the indirect shape implementation using index buffer, then it's buffer must also has
  /// storage usage and can be accessed here. This api is to expose the ability to convert the
  /// draw index call into draw array in some case.
  ///
  /// Return None if the id not matched
  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>>;

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      hasher.hash(self.as_any().type_id());
    })
  }

  fn as_any(&self) -> &dyn Any;
}

impl IndirectDrawProviderCreator for Vec<Box<dyn IndirectModelShapeRenderImpl>> {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    for provider in self {
      if let Some(v) = provider.get_impl_distinguish_key_by_impl_select_id(id) {
        return Some(v);
      }
    }
    None
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    dispatch_info_device_offset_compacted: &MultiRangeDispatchInfo,
    id: RawEntityHandle,
  ) -> Option<Vec<Box<dyn IndirectDrawProvider>>> {
    cx.next_scope_index();
    for (i, provider) in self.iter().enumerate() {
      if let Some(v) = cx.keyed_scope(&i, |cx| {
        provider.use_create_or_update_indirect_draw_providers(
          cx,
          list,
          dispatch_info_device_offset_compacted,
          id,
        )
      }) {
        return Some(v);
      }
    }
    None
  }
}

impl DrawCommandBuilderCreator for Vec<Box<dyn IndirectModelShapeRenderImpl>> {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    for provider in self {
      if let Some(v) = provider.make_draw_command_builder(id) {
        return Some(v);
      }
    }
    None
  }
}

impl IndirectModelShapeRenderImpl for Vec<Box<dyn IndirectModelShapeRenderImpl>> {
  fn make_component_indirect(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(com) = provider.make_component_indirect(idx) {
        return Some(com);
      }
    }
    None
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    for provider in self {
      if let Some(v) = provider.hash_shader_group_key_with_self_type_info(any_id, hasher) {
        return Some(v);
      }
    }
    None
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>> {
    for provider in self {
      if let Some(v) = provider.get_index_storage_buffer(any_idx) {
        return Some(v);
      }
    }
    None
  }
}
