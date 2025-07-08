use rendiation_webgpu_midc_downgrade::*;

use crate::*;

pub struct MIDCDowngradeBatch {
  pub helper: DowngradeMultiIndirectDrawCountHelper,
  pub cmd: DrawCommand,
}

impl ShaderHashProvider for MIDCDowngradeBatch {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.helper.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for MIDCDowngradeBatch {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    self.helper.bind(&mut cx.binding);
  }
}

impl IndirectDrawProvider for MIDCDowngradeBatch {
  fn create_indirect_invocation_source(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource> {
    Box::new(self.helper.build(binding))
  }

  fn draw_command(&self) -> DrawCommand {
    self.cmd.clone()
  }
}

impl IndirectBatchInvocationSource for DowngradeMultiIndirectDrawCountHelperInvocation {
  fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
    let vertex_index = builder.query::<VertexIndex>();

    let MultiDrawDowngradeVertexInfo {
      sub_draw_command_idx: _,
      vertex_index_inside_sub_draw,
      base_vertex_or_index_offset_for_sub_draw,
      base_instance,
    } = self.get_current_vertex_draw_info(vertex_index);

    builder.register::<VertexIndexForMIDCDowngrade>(
      vertex_index_inside_sub_draw + base_vertex_or_index_offset_for_sub_draw,
    );

    base_instance
  }
}

only_vertex!(VertexIndexForMIDCDowngrade, u32);
