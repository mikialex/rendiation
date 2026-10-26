use rendiation_webgpu_midc_downgrade::*;

use crate::*;

/// The draw provider of the downgraded multi indirect draw count, the draw id is read by the
/// helper.
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
    Box::new(MIDCDowngradeBatchInvocation(self.helper.build(binding)))
  }

  fn draw_command(&self) -> DrawCommand {
    self.cmd.clone()
  }
}

struct MIDCDowngradeBatchInvocation(DowngradeMultiIndirectDrawCountHelperInvocation);

impl IndirectBatchInvocationSource for MIDCDowngradeBatchInvocation {
  fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
    self.0.current_invocation_scene_model_id(builder)
  }
}
