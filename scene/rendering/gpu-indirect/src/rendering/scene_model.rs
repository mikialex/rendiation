use crate::*;

pub trait IndirectBatchSceneModelRenderer: SceneModelRenderer {
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
      self.as_any().type_id().hash(hasher);
    })
  }

  fn as_any(&self) -> &dyn Any;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder>;
}

pub struct IndirectPreferredComOrderRendererProvider {
  pub ids: DefaultSceneModelIdProvider,
  pub node: BoxedQueryBasedGPUFeature<Box<dyn IndirectNodeRenderImpl>>,
  pub model_impl: Vec<BoxedQueryBasedGPUFeature<Box<dyn IndirectModelRenderImpl>>>,
}

impl QueryBasedFeature<Box<dyn IndirectBatchSceneModelRenderer>>
  for IndirectPreferredComOrderRendererProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.node.register(qcx, cx);
    self.ids.register(qcx, cx);
    self.model_impl.iter_mut().for_each(|i| i.register(qcx, cx));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    self.node.deregister(qcx);
    self.ids.deregister(qcx);
    self.model_impl.iter_mut().for_each(|i| i.deregister(qcx));
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectBatchSceneModelRenderer> {
    Box::new(IndirectPreferredComOrderRenderer {
      model_impl: self.model_impl.iter().map(|i| i.create_impl(cx)).collect(),
      node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      node_render: self.node.create_impl(cx),
      id_inject: self.ids.create_impl(cx),
    })
  }
}

pub struct IndirectPreferredComOrderRenderer {
  model_impl: Vec<Box<dyn IndirectModelRenderImpl>>,
  node_render: Box<dyn IndirectNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
  id_inject: DefaultSceneModelIdInject,
}

impl SceneModelRenderer for IndirectPreferredComOrderRenderer {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    let scene_model_id = create_uniform(idx.alloc_index(), &cx.gpu.device);
    let cmd = self
      .make_draw_command_builder(idx)
      .unwrap()
      .draw_command_host_access(idx);

    struct SingleModelImmediateDraw {
      scene_model_id: UniformBufferDataView<u32>,
      cmd: DrawCommand,
    }

    impl ShaderHashProvider for SingleModelImmediateDraw {
      shader_hash_type_id! {}
    }

    impl ShaderPassBuilder for SingleModelImmediateDraw {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.scene_model_id);
      }
    }

    impl IndirectDrawProvider for SingleModelImmediateDraw {
      fn create_indirect_invocation_source(
        &self,
        binding: &mut ShaderBindGroupBuilder,
      ) -> Box<dyn IndirectBatchInvocationSource> {
        struct SingleModelImmediateDrawInvocation {
          scene_model_id: ShaderReadonlyPtrOf<u32>,
        }

        impl IndirectBatchInvocationSource for SingleModelImmediateDrawInvocation {
          fn current_invocation_scene_model_id(&self, _: &ShaderVertexBuilder) -> Node<u32> {
            self.scene_model_id.load()
          }
        }

        Box::new(SingleModelImmediateDrawInvocation {
          scene_model_id: binding.bind_by(&self.scene_model_id.clone()),
        })
      }

      fn draw_command(&self) -> DrawCommand {
        self.cmd.clone()
      }
    }

    self
      .render_indirect_batch_models(
        &SingleModelImmediateDraw {
          scene_model_id,
          cmd,
        },
        idx,
        camera,
        tex,
        pass,
        cx,
      )
      .unwrap();

    Ok(())
  }
}

impl IndirectBatchSceneModelRenderer for IndirectPreferredComOrderRenderer {
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectDrawProvider,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Option<()> {
    let id_inject = Box::new(self.id_inject.clone()) as Box<dyn RenderComponent>;

    let node = self.node.get(any_id)?;
    let node = self.node_render.make_component_indirect(node)?;

    let sub_id_injector = self.model_impl.device_id_injector(any_id)?;
    let shape = self.model_impl.shape_renderable_indirect(any_id)?;
    let material = self.model_impl.material_renderable_indirect(any_id, tex)?;

    let camera = Box::new(camera) as Box<dyn RenderComponent>;
    let pass = Box::new(pass) as Box<dyn RenderComponent>;
    let tex = Box::new(GPUTextureSystemAsRenderComponent(tex)) as Box<dyn RenderComponent>;
    let draw_source =
      Box::new(IndirectDrawProviderAsRenderComponent(models)) as Box<dyn RenderComponent>;

    let command = models.draw_command();

    let contents: [BindingController<Box<dyn RenderComponent>>; 9] = [
      draw_source.into_assign_binding_index(0),
      tex.into_assign_binding_index(0),
      pass.into_assign_binding_index(0),
      id_inject.into_assign_binding_index(0),
      sub_id_injector.into_assign_binding_index(2),
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

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    self.model_impl.make_draw_command_builder(any_idx)
  }
}

#[derive(Default)]
pub struct DefaultSceneModelIdProvider {
  pub id_buffer: QueryToken,
}

#[derive(Clone)]
pub struct DefaultSceneModelIdInject {
  pub id_buffer: StorageBufferReadonlyDataView<[SceneModelStorage]>,
}

impl QueryBasedFeature<DefaultSceneModelIdInject> for DefaultSceneModelIdProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.id_buffer = qcx.register_multi_updater(scene_model_data(cx));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.id_buffer);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> DefaultSceneModelIdInject {
    DefaultSceneModelIdInject {
      id_buffer: cx
        .take_multi_updater_updated::<CommonStorageBufferImpl<SceneModelStorage>>(self.id_buffer)
        .unwrap()
        .inner
        .gpu()
        .clone(),
    }
  }
}

impl ShaderHashProvider for DefaultSceneModelIdInject {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for DefaultSceneModelIdInject {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.id_buffer);
  }
}

impl GraphicsShaderProvider for DefaultSceneModelIdInject {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let buffer = binding.bind_by(&self.id_buffer);
      let current_id = builder.query::<LogicalRenderEntityId>();
      let model = buffer.index(current_id).load().expand();
      builder.register::<IndirectSceneNodeId>(model.node);
      builder.register::<IndirectSceneStdModelId>(model.std_model);
    })
  }
}
