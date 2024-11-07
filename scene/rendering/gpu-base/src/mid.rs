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
  fn current_invocation_scene_model_id(&self) -> Node<u32>;
}

impl DeviceSceneModelRenderBatch {
  pub fn create_indirect_draw_provider(
    &self,
    draw_command_builder: &dyn DrawCommandBuilder,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn IndirectDrawProvider> {
    Box::new(MultiIndirectDrawBatch {
      draw_command_buffer: todo!(),
    })
  }
}

struct MultiIndirectDrawBatch {
  draw_command_buffer: StorageBufferReadOnlyDataView<[DrawIndexedIndirect]>,
}

impl IndirectDrawProvider for MultiIndirectDrawBatch {
  fn create_indirect_invocation_source(&self) -> Box<dyn IndirectBatchInvocationSource> {
    struct MultiIndirectDrawBatchInvocation;

    impl IndirectBatchInvocationSource for MultiIndirectDrawBatchInvocation {
      fn current_invocation_scene_model_id(&self) -> Node<u32> {
        todo!()
      }
    }

    Box::new(MultiIndirectDrawBatchInvocation)
  }

  fn draw_command(&self) -> DrawCommand {
    todo!()
    // DrawCommand::MultiIndirect { indexed: true, indirect_buffer: (), indirect_offset: 0, count: () }
  }
}

impl ShaderPassBuilder for MultiIndirectDrawBatch {}
impl ShaderHashProvider for MultiIndirectDrawBatch {
  shader_hash_type_id! {}
}
