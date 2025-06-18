use crate::*;

mod indexed;
pub use indexed::*;

mod none_indexed;
pub use none_indexed::*;

mod midc_downgrade;
pub use midc_downgrade::*;

pub enum DrawCommandBuilder {
  Indexed(Box<dyn IndexedDrawCommandBuilder>),
  NoneIndexed(Box<dyn NoneIndexedDrawCommandBuilder>),
}

impl DrawCommandBuilder {
  pub fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    match self {
      DrawCommandBuilder::Indexed(builder) => builder.draw_command_host_access(id),
      DrawCommandBuilder::NoneIndexed(builder) => builder.draw_command_host_access(id),
    }
  }
}

pub trait IndirectDrawProvider: ShaderHashProvider + ShaderPassBuilder {
  fn create_indirect_invocation_source(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource>;
  fn draw_command(&self) -> DrawCommand;
}

pub trait IndirectBatchInvocationSource {
  fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32>;
  fn extra_register(&self, _builder: &mut ShaderVertexBuilder) {}
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
      invocation.extra_register(builder);
    })
  }
}

impl DeviceSceneModelRenderSubBatch {
  pub fn create_default_indirect_draw_provider(
    &self,
    draw_command_builder: DrawCommandBuilder,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn IndirectDrawProvider> {
    match draw_command_builder {
      DrawCommandBuilder::Indexed(generator) => {
        let generator = IndexedDrawCommandGenerator {
          scene_models: self.scene_models.clone(),
          generator,
        };
        let size = generator.result_size();

        let init = ZeroedArrayByArrayLength(size as usize);
        let draw_command_buffer = StorageBufferDataView::create_by_with_extra_usage(
          cx.gpu.device.as_ref(),
          StorageBufferInit::<[DrawIndexedIndirect]>::from(init),
          BufferUsages::INDIRECT,
        );

        let r = generator.materialize_storage_buffer_into(draw_command_buffer, cx);
        let draw_command_buffer = MultiIndirectDrawBatchDeviceCommands::Indexed(r.buffer);
        let draw_count = r.size.unwrap_or_else(|| {
          StorageBufferReadonlyDataView::create_by_with_extra_usage(
            &cx.gpu.device,
            StorageBufferInit::WithInit(&Vec4::new(size, 0, 0, 0)),
            BufferUsages::INDIRECT,
          )
        });

        if cx
          .gpu
          .info
          .supported_features
          .contains(Features::MULTI_DRAW_INDIRECT_COUNT)
        {
          Box::new(MultiIndirectDrawBatch {
            draw_command_buffer,
            draw_count,
          })
        } else {
          let cmd = DrawCommand::MultiIndirectCount {
            indexed: matches!(
              &draw_command_buffer,
              MultiIndirectDrawBatchDeviceCommands::Indexed(_)
            ),
            indirect_buffer: draw_command_buffer.indirect_buffer().clone(),
            indirect_count: draw_count.gpu.clone(),
            max_count: draw_command_buffer.item_count(),
          };

          let (helper, cmd) =
            rendiation_webgpu_midc_downgrade::downgrade_multi_indirect_draw_count(cmd, cx);
          Box::new(MIDCDowngradeBatch { helper, cmd })
        }
      }
      DrawCommandBuilder::NoneIndexed(generator) => {
        let generator = NoneIndexedDrawCommandGenerator {
          scene_models: self.scene_models.clone(),
          generator,
        };
        let size = generator.result_size();

        let init = ZeroedArrayByArrayLength(size as usize);
        let draw_command_buffer = StorageBufferDataView::create_by_with_extra_usage(
          cx.gpu.device.as_ref(),
          StorageBufferInit::<[DrawIndirect]>::from(init),
          BufferUsages::INDIRECT,
        );

        let r = generator.materialize_storage_buffer_into(draw_command_buffer, cx);

        Box::new(MultiIndirectDrawBatch {
          draw_command_buffer: MultiIndirectDrawBatchDeviceCommands::NoneIndexed(r.buffer),
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
  }
}

struct MultiIndirectDrawBatch {
  draw_command_buffer: MultiIndirectDrawBatchDeviceCommands,
  draw_count: StorageBufferReadonlyDataView<Vec4<u32>>,
}

enum MultiIndirectDrawBatchDeviceCommands {
  Indexed(StorageBufferReadonlyDataView<[DrawIndexedIndirect]>),
  NoneIndexed(StorageBufferReadonlyDataView<[DrawIndirect]>),
}

impl MultiIndirectDrawBatchDeviceCommands {
  fn item_count(&self) -> u32 {
    match self {
      MultiIndirectDrawBatchDeviceCommands::Indexed(buffer) => buffer.item_count(),
      MultiIndirectDrawBatchDeviceCommands::NoneIndexed(buffer) => buffer.item_count(),
    }
  }
  fn indirect_buffer(&self) -> &GPUBufferResourceView {
    match self {
      MultiIndirectDrawBatchDeviceCommands::Indexed(buffer) => &buffer.gpu,
      MultiIndirectDrawBatchDeviceCommands::NoneIndexed(buffer) => &buffer.gpu,
    }
  }
}

impl IndirectDrawProvider for MultiIndirectDrawBatch {
  fn create_indirect_invocation_source(
    &self,
    _: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource> {
    struct MultiIndirectDrawBatchInvocation;

    impl IndirectBatchInvocationSource for MultiIndirectDrawBatchInvocation {
      fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
        builder.query::<VertexInstanceIndex>()
      }
    }

    Box::new(MultiIndirectDrawBatchInvocation)
  }

  fn draw_command(&self) -> DrawCommand {
    DrawCommand::MultiIndirectCount {
      indexed: matches!(
        &self.draw_command_buffer,
        MultiIndirectDrawBatchDeviceCommands::Indexed(_)
      ),
      indirect_buffer: self.draw_command_buffer.indirect_buffer().clone(),
      indirect_count: self.draw_count.gpu.clone(),
      max_count: self.draw_command_buffer.item_count(),
    }
  }
}

impl ShaderPassBuilder for MultiIndirectDrawBatch {}
impl ShaderHashProvider for MultiIndirectDrawBatch {
  shader_hash_type_id! {}
}
