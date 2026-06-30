use crate::*;

pub trait IndirectModelRenderImpl: IndirectDrawProviderCreator + DrawCommandBuilderCreator {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      hasher.hash(self.as_any().type_id());
    })
  }

  fn as_any(&self) -> &dyn Any;

  /// This is the place to provide self's render component implementation
  ///
  /// This id inject logic is not necessary if the subsequent render component implementation not required
  /// any id info, but the implementation is still required to return Some(Box::new(())) as the return value.
  fn device_id_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn shape_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;

  /// see [IndirectModelShapeRenderImpl::get_index_storage_buffer]
  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>>;

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
}

impl IndirectDrawProviderCreator for Vec<Box<dyn IndirectModelRenderImpl>> {
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

impl DrawCommandBuilderCreator for Vec<Box<dyn IndirectModelRenderImpl>> {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    for provider in self {
      if let Some(v) = provider.make_draw_command_builder(id) {
        return Some(v);
      }
    }
    None
  }
}

impl IndirectModelRenderImpl for Vec<Box<dyn IndirectModelRenderImpl>> {
  fn device_id_injector(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(v) = provider.device_id_injector(any_id) {
        return Some(v);
      }
    }
    None
  }
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
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

  fn shape_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(v) = provider.shape_renderable_indirect(any_idx, cx) {
        return Some(v);
      }
    }
    None
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>> {
    for provider in self {
      if let Some(v) = provider.get_index_storage_buffer(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(v) = provider.material_renderable_indirect(any_idx, cx) {
        return Some(v);
      }
    }
    None
  }
}

pub fn use_std_model_renderer(
  cx: &mut QueryGPUHookCx,
  materials: Option<Box<dyn IndirectModelMaterialRenderImpl>>,
  material_key: UseResult<impl DataChanges<Key = u32, Value = u32> + 'static>,
  shapes: Option<Box<dyn IndirectModelShapeRenderImpl>>,
  revere_z: bool,
) -> Option<SceneStdModelIndirectRenderer> {
  let (cx, std_model) = cx.use_storage_buffer("std model metadata", 128, u32::MAX);

  cx.use_changes::<StandardModelRefAttributesMeshEntity>()
    .map(|mesh| mesh.map_u32_index_or_u32_max())
    .update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, mesh));

  cx.use_changes::<StandardModelRefSkin>()
    .map(|mesh| mesh.map_u32_index_or_u32_max())
    .update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, skin));

  let state_override = use_state_overrides(cx, revere_z);

  material_key.update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, material));

  std_model.use_update(cx);
  std_model.use_max_item_count_by_db_entity::<StandardModelEntity>(cx);

  cx.when_render(|| SceneStdModelIndirectRenderer {
    model: read_global_db_foreign_key(),
    materials: materials.unwrap(),
    shapes: shapes.unwrap(),
    std_model: std_model.get_gpu_buffer(),
    states: state_override.unwrap(),
  })
}

pub struct SceneStdModelIndirectRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  std_model: AbstractReadonlyStorageBuffer<[SceneStdModelStorage]>,
  materials: Box<dyn IndirectModelMaterialRenderImpl>,
  shapes: Box<dyn IndirectModelShapeRenderImpl>,
  states: StateOverrides,
}

impl IndirectDrawProviderCreator for SceneStdModelIndirectRenderer {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let model = self.model.get(id)?;
    self
      .shapes
      .get_impl_distinguish_key_by_impl_select_id(model.into_raw())
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    dispatch_info_device_offset_compacted: &MultiRangeDispatchInfo,
    id: RawEntityHandle,
  ) -> Option<Vec<Box<dyn IndirectDrawProvider>>> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let model = self.model.get(id)?;
    self.shapes.use_create_or_update_indirect_draw_providers(
      cx,
      list,
      dispatch_info_device_offset_compacted,
      model.into_raw(),
    )
  }
}

impl DrawCommandBuilderCreator for SceneStdModelIndirectRenderer {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let model = self.model.get(id)?;
    self.shapes.make_draw_command_builder(model.into_raw())
  }
}

impl IndirectModelRenderImpl for SceneStdModelIndirectRenderer {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let model = self.model.get(any_id)?;
    self
      .materials
      .hash_shader_group_key_with_self_type_info(model, hasher)?;
    self
      .shapes
      .hash_shader_group_key_with_self_type_info(model, hasher)?;
    self.states.get_gpu(model)?.hash_pipeline(hasher);
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn device_id_injector(
    &self,
    sm: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    struct SceneStdModelIdInjector<'a> {
      std_model: AbstractReadonlyStorageBuffer<[SceneStdModelStorage]>,
      states: StateGPUImpl<'a>,
    }

    impl<'a> ShaderHashProvider for SceneStdModelIdInjector<'a> {
      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.states.hash_pipeline(hasher);
      }
      shader_hash_type_id! {SceneStdModelIdInjector<'static>}
    }

    impl<'a> ShaderPassBuilder for SceneStdModelIdInjector<'a> {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.std_model);
      }
    }

    impl<'a> GraphicsShaderProvider for SceneStdModelIdInjector<'a> {
      fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.vertex(|builder, binding| {
          let buffer = binding.bind_by(&self.std_model);
          let sm_id = builder.query::<IndirectSceneStdModelId>();
          let info = buffer.index(sm_id).load().expand();
          builder.register::<IndirectAbstractMaterialId>(info.material);
          builder.register::<IndirectSkinId>(info.skin);
          builder.set_vertex_out::<IndirectAbstractMaterialId>(info.material);
          builder.register::<IndirectAbstractMeshId>(info.mesh);
        });
        self.states.build(builder);
      }
    }

    let model = self.model.get(sm)?;
    Some(Box::new(SceneStdModelIdInjector {
      std_model: self.std_model.clone(),
      states: self.states.get_gpu(model)?,
    }))
  }

  fn shape_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_component_indirect(model)
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let model = self.model.get(any_idx)?;
    self.materials.make_component_indirect(model, cx)
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>> {
    let model_id = self.model.get(any_idx)?;
    self.shapes.get_index_storage_buffer(model_id)
  }
}

only_vertex!(IndirectSceneStdModelId, u32);

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct SceneStdModelStorage {
  pub mesh: u32, // todo, improve: this data is duplicate with the mesh dispatcher sm-ref-mesh data
  pub material: u32,
  pub skin: u32,
}

only_vertex!(IndirectSkinId, u32);
