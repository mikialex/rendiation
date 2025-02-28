use crate::*;

pub trait DrawCommandBuilder: ShaderHashProvider + ShaderPassBuilder + DynClone {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand;
  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DrawCommandBuilderInvocation>;

  fn bind(&self, builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(DrawCommandBuilder);

pub trait DrawCommandBuilderInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirect>;
}

pub trait IndirectDrawProvider: ShaderHashProvider + ShaderPassBuilder {
  fn create_indirect_invocation_source(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource>;
  fn draw_command(&self) -> DrawCommand;
}

pub struct IndirectDrawProviderAsRenderComponent<'a>(pub &'a dyn IndirectDrawProvider);

impl ShaderHashProvider for IndirectDrawProviderAsRenderComponent<'_> {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.0.hash_type_info(hasher)
  }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for IndirectDrawProviderAsRenderComponent<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.setup_pass(ctx);
  }
}

impl GraphicsShaderProvider for IndirectDrawProviderAsRenderComponent<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binder| {
      let invocation = self.0.create_indirect_invocation_source(binder);
      let scene_model_id = invocation.current_invocation_scene_model_id(builder);
      builder.register::<LogicalRenderEntityId>(scene_model_id);
    })
  }
}

pub trait IndirectBatchInvocationSource {
  fn current_invocation_scene_model_id(&self, builder: &ShaderVertexBuilder) -> Node<u32>;
}

impl DeviceSceneModelRenderSubBatch {
  pub fn create_indirect_draw_provider(
    &self,
    draw_command_builder: Box<dyn DrawCommandBuilder>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn IndirectDrawProvider> {
    let generator = DrawCommandGenerator {
      scene_models: self.scene_models.clone(),
      generator: draw_command_builder,
    };
    let size = generator.result_size();

    let init = ZeroedArrayByArrayLength(size as usize);
    let draw_command_buffer = StorageBufferDataView::create_by_with_extra_usage(
      cx.gpu.device.as_ref(),
      StorageBufferInit::<[DrawIndexedIndirect]>::from(init),
      BufferUsages::INDIRECT,
    );

    let r = generator.materialize_storage_buffer_into(draw_command_buffer, cx);

    Box::new(MultiIndirectDrawBatch {
      draw_command_buffer: r.buffer,
      draw_count: r.size.unwrap_or_else(|| {
        StorageBufferReadonlyDataView::create_by_with_extra_usage(
          &cx.gpu.device,
          StorageBufferInit::WithInit(&Vec4::new(size, 0, 0, 0)),
          BufferUsages::INDIRECT,
        )
      }),
    })
  }
}

struct MultiIndirectDrawBatch {
  draw_command_buffer: StorageBufferReadonlyDataView<[DrawIndexedIndirect]>,
  draw_count: StorageBufferReadonlyDataView<Vec4<u32>>,
}

impl IndirectDrawProvider for MultiIndirectDrawBatch {
  fn create_indirect_invocation_source(
    &self,
    _: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource> {
    struct MultiIndirectDrawBatchInvocation;

    impl IndirectBatchInvocationSource for MultiIndirectDrawBatchInvocation {
      fn current_invocation_scene_model_id(&self, builder: &ShaderVertexBuilder) -> Node<u32> {
        builder.query::<VertexInstanceIndex>()
      }
    }

    Box::new(MultiIndirectDrawBatchInvocation)
  }

  fn draw_command(&self) -> DrawCommand {
    DrawCommand::MultiIndirectCount {
      indexed: true,
      indirect_buffer: self.draw_command_buffer.gpu.clone(),
      indirect_count: self.draw_count.gpu.clone(),
      max_count: self.draw_command_buffer.item_count(),
    }
  }
}

impl ShaderPassBuilder for MultiIndirectDrawBatch {}
impl ShaderHashProvider for MultiIndirectDrawBatch {
  shader_hash_type_id! {}
}

#[derive(Clone)]
struct DrawCommandGenerator {
  scene_models: Box<dyn DeviceParallelComputeIO<u32>>,
  generator: Box<dyn DrawCommandBuilder>,
}

impl DeviceParallelCompute<Node<DrawIndexedIndirect>> for DrawCommandGenerator {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<DrawIndexedIndirect>>> {
    Box::new(DrawCommandGeneratorComponent {
      scene_models: self.scene_models.execute_and_expose(cx),
      generator: self.generator.clone(),
    })
  }

  fn result_size(&self) -> u32 {
    self.scene_models.result_size()
  }
}
impl DeviceParallelComputeIO<DrawIndexedIndirect> for DrawCommandGenerator {}

struct DrawCommandGeneratorComponent {
  scene_models: Box<dyn DeviceInvocationComponent<Node<u32>>>,
  generator: Box<dyn DrawCommandBuilder>,
}

impl ShaderHashProvider for DrawCommandGeneratorComponent {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.scene_models.hash_pipeline(hasher);
    self.generator.hash_pipeline(hasher);
  }
}

impl DeviceInvocationComponent<Node<DrawIndexedIndirect>> for DrawCommandGeneratorComponent {
  fn work_size(&self) -> Option<u32> {
    self.scene_models.work_size()
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<DrawIndexedIndirect>>> {
    Box::new(DrawCommandGeneratorInvocation {
      scene_models: self.scene_models.build_shader(builder),
      generator: self.generator.build_invocation(builder),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.scene_models.bind_input(builder);
    self.generator.bind(builder);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    self.scene_models.requested_workgroup_size()
  }
}

struct DrawCommandGeneratorInvocation {
  scene_models: Box<dyn DeviceInvocation<Node<u32>>>,
  generator: Box<dyn DrawCommandBuilderInvocation>,
}

impl DeviceInvocation<Node<DrawIndexedIndirect>> for DrawCommandGeneratorInvocation {
  fn invocation_logic(
    &self,
    logic_global_id: Node<Vec3<u32>>,
  ) -> (Node<DrawIndexedIndirect>, Node<bool>) {
    let (id, valid) = self.scene_models.invocation_logic(logic_global_id);

    let draw_command = make_local_var::<DrawIndexedIndirect>();
    if_by(valid, || {
      draw_command.store(self.generator.generate_draw_command(id));
    });

    (draw_command.load(), valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.scene_models.invocation_size()
  }
}
