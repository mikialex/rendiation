use crate::*;

pub fn use_indirect_scene_model(
  cx: &mut QueryGPUHookCx,
  model_impl: Option<Box<dyn IndirectModelRenderImpl>>,
) -> Option<IndirectPreferredComOrderRenderer> {
  let node = use_node_storage(cx);

  let id_inject = cx
    .use_storage_buffer(scene_model_data)
    .map(DefaultSceneModelIdInject);

  cx.when_render(|| IndirectPreferredComOrderRenderer {
    model_impl: model_impl.unwrap(),
    node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
    node_render: node.unwrap(),
    id_inject: id_inject.unwrap(),
  })
}

pub trait IndirectBatchSceneModelRenderer: SceneModelRenderer {
  /// note, this interface can not be merged with [IndirectBatchSceneModelRenderer::render_indirect_batch_models]
  /// because render_indirect_batch_models will be called inside active renderpass, at that time,
  /// the encoder will be used by the renderpass exclusively.
  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Box<dyn IndirectDrawProvider>;

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

pub struct IndirectPreferredComOrderRenderer {
  model_impl: Box<dyn IndirectModelRenderImpl>,
  node_render: IndirectNodeRenderer,
  node: ForeignKeyReadView<SceneModelRefNode>,
  id_inject: DefaultSceneModelIdInject,
}

impl SceneModelRenderer for IndirectPreferredComOrderRenderer {
  /// The implementation will try directly create a single draw
  /// For some advance implementation, this may failed because it requires
  /// extra compute shader prepare logic, which is impossible to placed here
  /// because the render pass is active.
  ///
  /// If we invent something like preflight encoder, and submit prepare work
  /// on it, this is possible, but from the perspective of performance, this is
  /// meaningless. so the current behavior is we will always failed on some advance
  /// implementation here.
  ///
  /// todo, consider buffer the call and submit later?
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
          fn current_invocation_scene_model_id(&self, _: &mut ShaderVertexBuilder) -> Node<u32> {
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
  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Box<dyn IndirectDrawProvider> {
    let draw_command_builder = self
      .make_draw_command_builder(batch.impl_select_id)
      .unwrap();

    ctx.access_parallel_compute(|cx| {
      batch.create_default_indirect_draw_provider(draw_command_builder, cx)
    })
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
    let id_inject = &self.id_inject as &dyn RenderComponent;

    let node = self.node.get(any_id)?;
    let node = self.node_render.make_component_indirect(node)?;
    let node = node.as_ref();

    let sub_id_injector = self.model_impl.device_id_injector(any_id)?;
    let sub_id_injector = sub_id_injector.as_ref();

    let shape = self.model_impl.shape_renderable_indirect(any_id)?;
    let shape = shape.as_ref();

    let material = self.model_impl.material_renderable_indirect(any_id, tex)?;
    let material = material.as_ref();

    let camera = camera as &dyn RenderComponent;
    let pass = pass as &dyn RenderComponent;
    let tex = &GPUTextureSystemAsRenderComponent(tex) as &dyn RenderComponent;
    let draw_source = &IndirectDrawProviderAsRenderComponent(models) as &dyn RenderComponent;

    let command = models.draw_command();

    let contents: [BindingController<&dyn RenderComponent>; 9] = [
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

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[derive(Clone)]
pub struct DefaultSceneModelIdInject(StorageBufferReadonlyDataView<[SceneModelStorage]>);

impl ShaderHashProvider for DefaultSceneModelIdInject {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for DefaultSceneModelIdInject {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.0);
  }
}

impl GraphicsShaderProvider for DefaultSceneModelIdInject {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let buffer = binding.bind_by(&self.0);
      let current_id = builder.query::<LogicalRenderEntityId>();
      let model = buffer.index(current_id).load().expand();
      builder.register::<IndirectSceneNodeId>(model.node);
      builder.register::<IndirectSceneStdModelId>(model.std_model);
    })
  }
}

pub type SceneModelStorageBuffer = ReactiveStorageBufferContainer<SceneModelStorage>;

pub fn scene_model_data(cx: &GPU) -> SceneModelStorageBuffer {
  let std_model = global_watch()
    .watch::<SceneModelStdModelRenderPayload>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .into_query_update_storage(offset_of!(SceneModelStorage, std_model));

  let node = global_watch()
    .watch::<SceneModelRefNode>()
    .collective_filter_map(|id| id.map(|v| v.index()))
    .into_query_update_storage(offset_of!(SceneModelStorage, node));

  create_reactive_storage_buffer_container(128, u32::MAX, cx)
    .with_source(std_model)
    .with_source(node)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct, Debug)]
pub struct SceneModelStorage {
  pub node: u32,
  pub std_model: u32,
}

pub struct SceneModelGPUStorage<'a> {
  pub buffer: &'a SceneModelStorageBuffer,
}

impl ShaderHashProvider for SceneModelGPUStorage<'_> {
  shader_hash_type_id! {SceneModelGPUStorage<'static>}
}

impl GraphicsShaderProvider for SceneModelGPUStorage<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let models = binding.bind_by(self.buffer.inner.gpu());
      let current_model_id = builder.query::<LogicalRenderEntityId>();
      let model = models.index(current_model_id).load().expand();

      builder.register::<IndirectSceneNodeId>(model.node);
      builder.register::<IndirectSceneStdModelId>(model.std_model);
    })
  }
}

impl ShaderPassBuilder for SceneModelGPUStorage<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
