use crate::*;

pub trait DrawCommandBuilder: ShaderHashProvider + ShaderPassBuilder {
  fn build_invocation(&self) -> Box<dyn DrawCommandBuilderInvocation>;
}

pub trait DrawCommandBuilderInvocation {
  fn generate_draw_command(
    &self,
    mesh_handle: Node<u32>,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirect>;
}

pub trait IndirectDrawProvider: ShaderHashProvider + ShaderPassBuilder {
  fn create_indirect_invocation_source(&self) -> Box<dyn IndirectBatchInvocationSource>;
  fn draw_command(&self) -> DrawCommand;
}

pub trait IndirectBatchInvocationSource {
  fn current_invocation_scene_model_id(&self, builder: &ShaderVertexBuilder) -> Node<u32>;
}

impl DeviceSceneModelRenderSubBatch {
  pub fn create_indirect_draw_provider(
    &self,
    draw_command_builder: &dyn DrawCommandBuilder,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn IndirectDrawProvider + 'static> {
    Box::new(MultiIndirectDrawBatch {
      draw_command_buffer: todo!(),
      draw_count: todo!(),
    })
  }
}

struct MultiIndirectDrawBatch {
  draw_command_buffer: StorageBufferReadOnlyDataView<[DrawIndexedIndirect]>,
  draw_count: StorageBufferReadOnlyDataView<u32>,
}

impl IndirectDrawProvider for MultiIndirectDrawBatch {
  fn create_indirect_invocation_source(&self) -> Box<dyn IndirectBatchInvocationSource> {
    struct MultiIndirectDrawBatchInvocation;

    impl IndirectBatchInvocationSource for MultiIndirectDrawBatchInvocation {
      fn current_invocation_scene_model_id(&self, builder: &ShaderVertexBuilder) -> Node<u32> {
        builder.query::<VertexInstanceIndex>().unwrap()
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
