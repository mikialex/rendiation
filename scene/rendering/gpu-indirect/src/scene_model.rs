use crate::*;

pub fn use_indirect_scene_model(
  cx: &mut QueryGPUHookCx,
  node_impl: Option<Box<dyn IndirectNodeRenderImpl>>,
  model_impl: Option<Box<dyn IndirectModelRenderImpl>>,
  force_midc_downgrade: bool,
) -> Option<IndirectPreferredComOrderRenderer> {
  let sm_to_node_device = use_db_device_foreign_key::<SceneModelRefNode>(cx);

  cx.when_render(|| IndirectPreferredComOrderRenderer {
    model_impl: model_impl.unwrap(),
    node: read_global_db_foreign_key(),
    node_render: node_impl.unwrap(),
    id_inject: DefaultSceneModelIdInject {
      sm_to_node: sm_to_node_device.unwrap(),
    },
    enable_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
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
  ) -> Option<Box<dyn IndirectDrawProvider>>;

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

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder>;
}

pub struct IndirectPreferredComOrderRenderer {
  model_impl: Box<dyn IndirectModelRenderImpl>,
  node_render: Box<dyn IndirectNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
  id_inject: DefaultSceneModelIdInject,
  enable_midc_downgrade: bool,
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
    let scene_model_id = create_uniform(idx.alloc_index(), &cx.gpu.device, "scene model id");
    let cmd = self
      .make_draw_command_builder(idx)
      .unwrap()
      .draw_command_host_access(idx)
      .ok_or(UnableToRenderSceneModelError::MeshBufferFailedGetDrawCommand)?;

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
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    self.model_impl.generate_indirect_draw_provider(batch, ctx)
    // .expect("unable to fine suitable indirect draw provider for this indirect draw batch")
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

    let model_info = self.model_impl.model_info_injector(any_id)?;
    let model_info = model_info.as_ref();

    let shape = self.model_impl.shape_renderable_indirect(any_id, tex)?;
    let shape = shape.as_ref();

    let material = self.model_impl.material_renderable_indirect(any_id, tex)?;
    let material = material.as_ref();

    let midc_index_downgrade = if self.enable_midc_downgrade {
      let index = self.model_impl.get_index_storage_buffer(any_id)?;
      let override_ = MidcDowngradeWrapperForIndirectMeshSystem { index };
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

    let contents: [BindingController<&dyn RenderComponent>; 10] = [
      draw_source.into_assign_binding_index(1),
      tex.into_assign_binding_index(0),
      pass.into_assign_binding_index(1),
      id_inject.into_assign_binding_index(0),
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
pub struct DefaultSceneModelIdInject {
  sm_to_node: AbstractReadonlyStorageBuffer<[u32]>,
}

impl ShaderHashProvider for DefaultSceneModelIdInject {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for DefaultSceneModelIdInject {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.sm_to_node);
  }
}

impl GraphicsShaderProvider for DefaultSceneModelIdInject {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let buffer = binding.bind_by(&self.sm_to_node);
      let current_id = builder.query::<LogicalRenderEntityId>();
      let node = buffer.index(current_id).load();
      builder.register::<IndirectSceneNodeId>(node);
    })
  }
}
