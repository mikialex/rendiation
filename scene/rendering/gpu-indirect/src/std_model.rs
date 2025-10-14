use crate::*;

pub trait IndirectModelRenderImpl {
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
      self.as_any().type_id().hash(hasher);
    })
  }

  fn as_any(&self) -> &dyn Any;

  /// this is actually place to provide self's render component implementation
  /// this id inject is not necessary if the implementation not required, but still required
  /// to return Some component.
  fn device_id_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder>;

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
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

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    for provider in self {
      if let Some(v) = provider.shape_renderable_indirect(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    for provider in self {
      if let Some(v) = provider.make_draw_command_builder(any_idx) {
        return Some(v);
      }
    }
    None
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    for provider in self {
      if let Some(v) = provider.generate_indirect_draw_provider(batch, ctx) {
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
  shapes: Option<Box<dyn IndirectModelShapeRenderImpl>>,
) -> Option<SceneStdModelIndirectRenderer> {
  let (cx, std_model) = cx.use_storage_buffer("std model metadata", 128, u32::MAX);

  cx.use_changes::<StandardModelRefAttributesMeshEntity>()
    .map(|mesh| mesh.map_u32_index_or_u32_max())
    .update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, mesh));

  cx.use_changes::<StandardModelRefSkin>()
    .map(|mesh| mesh.map_u32_index_or_u32_max())
    .update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, skin));

  let material_flat = cx.use_changes::<StandardModelRefUnlitMaterial>();
  let material_pbr_mr = cx.use_changes::<StandardModelRefPbrMRMaterial>();
  let material_pbr_sg = cx.use_changes::<StandardModelRefPbrSGMaterial>();

  let changes = if cx.is_spawning_stage() {
    UseResult::SpawnStageReady(SelectChanges([
      material_flat
        .expect_spawn_stage_ready()
        .map_some_u32_index(),
      material_pbr_mr
        .expect_spawn_stage_ready()
        .map_some_u32_index(),
      material_pbr_sg
        .expect_spawn_stage_ready()
        .map_some_u32_index(),
    ]))
  } else {
    UseResult::NotInStage
  };

  changes.update_storage_array(cx, std_model, offset_of!(SceneStdModelStorage, material));

  std_model.use_update(cx);
  std_model.use_max_item_count_by_db_entity::<StandardModelEntity>(cx);

  cx.when_render(|| SceneStdModelIndirectRenderer {
    model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
    materials: materials.unwrap(),
    shapes: shapes.unwrap(),
    std_model: std_model.get_gpu_buffer(),
  })
}

pub struct SceneStdModelIndirectRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  std_model: AbstractReadonlyStorageBuffer<[SceneStdModelStorage]>,
  materials: Box<dyn IndirectModelMaterialRenderImpl>,
  shapes: Box<dyn IndirectModelShapeRenderImpl>,
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
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn device_id_injector(
    &self,
    _: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    struct SceneStdModelIdInjector {
      std_model: AbstractReadonlyStorageBuffer<[SceneStdModelStorage]>,
    }

    impl ShaderHashProvider for SceneStdModelIdInjector {
      shader_hash_type_id! {}
    }

    impl ShaderPassBuilder for SceneStdModelIdInjector {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.std_model);
      }
    }

    impl GraphicsShaderProvider for SceneStdModelIdInjector {
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
      }
    }

    Some(Box::new(SceneStdModelIdInjector {
      std_model: self.std_model.clone(),
    }))
  }

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_component_indirect(model)
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    let model = self.model.get(any_idx)?;
    self.shapes.make_draw_command_builder(model)
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let model = self.model.get(any_idx)?;
    self.materials.make_component_indirect(model, cx)
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    let model_id = self.model.get(batch.impl_select_id)?;
    self
      .shapes
      .generate_indirect_draw_provider(batch, model_id, ctx)
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
