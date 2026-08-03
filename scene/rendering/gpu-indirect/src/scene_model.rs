use crate::*;

pub fn use_indirect_scene_model(
  cx: &mut QueryGPUHookCx,
  node_impl: Option<Box<dyn IndirectNodeRenderImpl>>,
  model_impl: Option<Box<dyn IndirectModelRenderImpl>>,
  force_midc_downgrade: bool,
) -> Option<IndirectPreferredComOrderRenderer> {
  cx.when_render(|| IndirectPreferredComOrderRenderer {
    model_impl: model_impl.unwrap(),
    node: read_global_db_foreign_key(),
    node_render: node_impl.unwrap(),
    enable_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
  })
}

pub trait IndirectBatchSceneModelRenderer:
  IndirectDrawProviderCreator + DrawCommandBuilderCreator
{
  /// the caller must guarantee the batch source can be drawn by the implementation selected by any_id
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectDrawProvider,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Option<()>;

  /// shader_group_key is like shader hash but a bit different
  ///
  /// - compute shader_group_key should be cheaper than shader hash
  ///   - the outside render dispatchers or component can be omitted
  ///   - the render component is not created
  /// - the shader_group_key logic must match the shader hash
  ///
  /// if error occurs, return None
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
}

pub struct IndirectPreferredComOrderRenderer {
  model_impl: Box<dyn IndirectModelRenderImpl>,
  node_render: Box<dyn IndirectNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
  enable_midc_downgrade: bool,
}

impl IndirectDrawProviderCreator for IndirectPreferredComOrderRenderer {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    self
      .model_impl
      .get_impl_distinguish_key_by_impl_select_id(id)
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    dispatch_info_device_offset_compacted: &MultiRangeDispatchInfo,
    id: RawEntityHandle,
  ) -> Option<Vec<Box<dyn IndirectDrawProvider>>> {
    self
      .model_impl
      .use_create_or_update_indirect_draw_providers(
        cx,
        list,
        dispatch_info_device_offset_compacted,
        id,
      )
  }
}

impl DrawCommandBuilderCreator for IndirectPreferredComOrderRenderer {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    self.model_impl.make_draw_command_builder(id)
  }
}

impl IndirectBatchSceneModelRenderer for IndirectPreferredComOrderRenderer {
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectDrawProvider,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Option<()> {
    let node = self.node.get(any_id)?;
    let node = self.node_render.make_component_indirect(node)?;
    let node = &NodeRenderComponent(node.as_ref()) as &dyn RenderComponent;

    let model_info = self.model_impl.model_info_injector(any_id)?;
    let model_info = model_info.as_ref();

    let shape = self.model_impl.shape_renderable_indirect(any_id, tex)?;
    let shape = shape.as_ref();

    let material = self.model_impl.material_renderable_indirect(any_id, tex)?;
    let material = material.as_ref();

    let midc_index_downgrade = if self.enable_midc_downgrade {
      let index = self.model_impl.get_index_storage_buffer(any_id)?;
      let override_ = MidcDowngradeWrapperForIndirectMeshSystem {
        index: index.map(|v| (v.buffer, v.should_access_as_u16)),
      };
      OptionRender(Some(Box::new(override_) as Box<dyn RenderComponent>))
    } else {
      OptionRender(None)
    };
    let midc_index_downgrade = &midc_index_downgrade as &dyn RenderComponent;

    let camera = camera as &dyn RenderComponent;
    let pass = pass as &dyn RenderComponent;
    let tex = &GPUTextureSystemAsRenderComponent(tex) as &dyn RenderComponent;
    let draw_source = &IndirectDrawProviderAsRenderComponent(models) as &dyn RenderComponent;

    let command = models.draw_command();

    let contents: [BindingController<&dyn RenderComponent>; _] = [
      draw_source.into_assign_binding_index(1),
      tex.into_assign_binding_index(0),
      pass.into_assign_binding_index(1),
      midc_index_downgrade.into_assign_binding_index(2),
      model_info.into_assign_binding_index(2),
      shape.into_assign_binding_index(2),
      node.into_assign_binding_index(2),
      camera.into_assign_binding_index(1),
      material.into_assign_binding_index(2),
    ];

    let render = Box::new(RenderArray(contents)) as Box<dyn RenderComponent>;
    render.render(cx, command);
    Some(())
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let node = self.node.get(any_id)?;
    self
      .node_render
      .hash_shader_group_key_with_self_type_info(node, hasher)?;
    self
      .model_impl
      .hash_shader_group_key_with_self_type_info(any_id, hasher)?;
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
