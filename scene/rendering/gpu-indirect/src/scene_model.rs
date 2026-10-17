use crate::*;

pub fn use_indirect_scene_model(
  cx: &mut QueryGPUHookCx,
  node_impl: Option<Box<dyn IndirectNodeRenderImpl>>,
  model_impl: Option<Box<dyn IndirectModelRendererResolver>>,
  force_midc_downgrade: bool,
) -> Option<IndirectPreferredComOrderRenderer> {
  cx.when_render(|| IndirectPreferredComOrderRenderer {
    model_impl: model_impl.unwrap(),
    node: read_global_db_foreign_key(),
    node_render: node_impl.unwrap(),
    enable_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
    scratch: Default::default(),
  })
}

/// The scene model renderer of the indirect renderer. The model impl is resolved once per call.
pub trait IndirectBatchSceneModelRenderer {
  /// the caller must guarantee the batch source can be drawn by the implementation selected by any_id
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectDrawProvider,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Result<(), UnableToRenderSceneModelError>;

  /// shader_group_key is like shader hash but a bit different
  ///
  /// - compute shader_group_key should be cheaper than shader hash
  ///   - the outside render dispatchers or component can be omitted
  ///   - the render component is not created
  /// - the shader_group_key logic must match the shader hash
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Result<(), UnableToRenderSceneModelError>;

  /// see [ResolvedIndirectModelRenderer::impl_distinguish_key]
  fn impl_distinguish_key(
    &self,
    impl_select_id: EntityHandle<SceneModelEntity>,
  ) -> Result<u64, UnableToRenderSceneModelError>;

  /// see [ResolvedIndirectModelRenderer::use_create_or_update_indirect_draw_providers]
  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    dispatch_info_device_offset_compacted: &MultiRangeDispatchInfo,
    impl_select_id: EntityHandle<SceneModelEntity>,
  ) -> Result<Vec<Box<dyn IndirectDrawProvider>>, UnableToRenderSceneModelError>;

  fn make_draw_command_builder(
    &self,
    id: EntityHandle<SceneModelEntity>,
  ) -> Result<DrawCommandBuilder, UnableToRenderSceneModelError>;
}

pub struct IndirectPreferredComOrderRenderer {
  model_impl: Box<dyn IndirectModelRendererResolver>,
  node_render: Box<dyn IndirectNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
  enable_midc_downgrade: bool,
  /// the resolved model renderers are allocated in it, it is reset after each call
  scratch: ScratchAllocator,
}

impl IndirectPreferredComOrderRenderer {
  fn resolve<'a>(
    &'a self,
    any_id: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Result<ResolvedIndirectModelRendererBox<'a>, UnableToRenderSceneModelError> {
    let model = self
      .model_impl
      .resolve(any_id, alloc)
      .ok_or(UnableToRenderSceneModelError::UnableToFindImpl { model_id: any_id })??;
    Ok(model)
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
  ) -> Result<(), UnableToRenderSceneModelError> {
    let err = SceneModelRenderDataError;
    self.node.get(any_id).ok_or(err("scene model node"))?;
    let node = self
      .node_render
      .make_component_indirect()
      .ok_or(err("node renderer"))?;
    let node = &NodeRenderComponent(node.as_ref()) as &dyn RenderComponent;

    self.scratch.scope(|alloc| {
      let model = self.resolve(any_id, alloc)?;

      let model_info = model.model_info_injector();
      let model_info = model_info.as_ref();

      let shape = model.shape_renderable_indirect(tex);
      let shape = shape.as_ref();

      let material = model.material_renderable_indirect(tex);
      let material = material.as_ref();

      let midc_index_downgrade = if self.enable_midc_downgrade {
        let index = model.get_index_storage_buffer();
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
      render.render(cx, RenderMethod::TraditionalDraw(command));
      Ok(())
    })
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Result<(), UnableToRenderSceneModelError> {
    let err = SceneModelRenderDataError;
    self.node.get(any_id).ok_or(err("scene model node"))?;
    self
      .node_render
      .hash_shader_group_key_with_self_type_info(hasher)
      .ok_or(err("node renderer group key"))?;
    self.scratch.scope(|alloc| {
      self
        .resolve(any_id, alloc)?
        .hash_shader_group_key_with_self_type_info(hasher);
      Ok(())
    })
  }

  fn impl_distinguish_key(
    &self,
    impl_select_id: EntityHandle<SceneModelEntity>,
  ) -> Result<u64, UnableToRenderSceneModelError> {
    self
      .scratch
      .scope(|alloc| Ok(self.resolve(impl_select_id, alloc)?.impl_distinguish_key()))
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    dispatch_info_device_offset_compacted: &MultiRangeDispatchInfo,
    impl_select_id: EntityHandle<SceneModelEntity>,
  ) -> Result<Vec<Box<dyn IndirectDrawProvider>>, UnableToRenderSceneModelError> {
    self.scratch.scope(|alloc| {
      let providers = self
        .resolve(impl_select_id, alloc)?
        .use_create_or_update_indirect_draw_providers(
          cx,
          list,
          dispatch_info_device_offset_compacted,
        )?;
      Ok(providers)
    })
  }

  fn make_draw_command_builder(
    &self,
    id: EntityHandle<SceneModelEntity>,
  ) -> Result<DrawCommandBuilder, UnableToRenderSceneModelError> {
    self
      .scratch
      .scope(|alloc| Ok(self.resolve(id, alloc)?.make_draw_command_builder()?))
  }
}
