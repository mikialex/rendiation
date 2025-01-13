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

  fn device_id_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>>;

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
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    for provider in self {
      if let Some(v) = provider.make_draw_command_builder(any_idx) {
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

pub struct DefaultSceneStdModelRendererProvider {
  pub std_model: UpdateResultToken,
  pub materials: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectModelMaterialRenderImpl>>>>,
  pub shapes: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectModelShapeRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn IndirectModelRenderImpl>> for DefaultSceneStdModelRendererProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.std_model = source.register_multi_updater(std_model_data(cx).inner);
    self
      .materials
      .iter_mut()
      .for_each(|p| p.register_resource(source, cx));
    self
      .shapes
      .iter_mut()
      .for_each(|p| p.register_resource(source, cx));
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.std_model);
    self
      .materials
      .iter_mut()
      .for_each(|p| p.deregister_resource(source));
    self
      .shapes
      .iter_mut()
      .for_each(|p| p.deregister_resource(source));
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn IndirectModelRenderImpl> {
    Box::new(SceneStdModelRenderer {
      model: global_entity_component_of::<SceneModelStdModelRenderPayload>().read_foreign_key(),
      materials: self.materials.iter().map(|v| v.create_impl(res)).collect(),
      shapes: self.shapes.iter().map(|v| v.create_impl(res)).collect(),
      std_model: res
        .take_multi_updater_updated::<CommonStorageBufferImpl<SceneStdModelStorage>>(self.std_model)
        .unwrap()
        .inner
        .gpu()
        .clone(),
    })
  }
}
struct SceneStdModelRenderer {
  model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
  std_model: StorageBufferReadOnlyDataView<[SceneStdModelStorage]>,
  materials: Vec<Box<dyn IndirectModelMaterialRenderImpl>>,
  shapes: Vec<Box<dyn IndirectModelShapeRenderImpl>>,
}

impl IndirectModelRenderImpl for SceneStdModelRenderer {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let model = self.model.get(any_id)?;
    self.materials.hash_shader_group_key(model, hasher)?;
    self.shapes.hash_shader_group_key(model, hasher)?;
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
      std_model: StorageBufferReadOnlyDataView<[SceneStdModelStorage]>,
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
  ) -> Option<Box<dyn DrawCommandBuilder>> {
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
}
