use rendiation_webgpu_midc_downgrade::*;

use crate::*;

/// assuming T using VertexInstanceIndex as draw id
pub struct MIDCDowngradeBatch<T> {
  pub helper: DowngradeMultiIndirectDrawCountHelper,
  pub cmd: DrawCommand,
  pub internal: T,
}

impl<T: ShaderHashProvider + 'static> ShaderHashProvider for MIDCDowngradeBatch<T> {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.helper.hash_pipeline(hasher);
    self.internal.hash_pipeline(hasher);
  }
}
impl<T: ShaderPassBuilder> ShaderPassBuilder for MIDCDowngradeBatch<T> {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    self.helper.bind(&mut cx.binding);
    self.internal.setup_pass(cx);
  }
}

impl<T: IndirectDrawProvider + 'static> IndirectDrawProvider for MIDCDowngradeBatch<T> {
  fn create_indirect_invocation_source(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource> {
    let source = DowngradeMultiIndirectDrawCountHelperInvocationWithBaseImpl {
      helper: self.helper.build(binding),
      base: self.internal.create_indirect_invocation_source(binding),
    };
    Box::new(source)
  }

  fn draw_command(&self) -> DrawCommand {
    self.cmd.clone()
  }
}

struct DowngradeMultiIndirectDrawCountHelperInvocationWithBaseImpl {
  helper: DowngradeMultiIndirectDrawCountHelperInvocation,
  base: Box<dyn IndirectBatchInvocationSource>,
}

impl IndirectBatchInvocationSource for DowngradeMultiIndirectDrawCountHelperInvocationWithBaseImpl {
  fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
    self.helper.current_invocation_scene_model_id(builder)
  }

  fn extra_register(&self, builder: &mut ShaderVertexBuilder) {
    self.base.extra_register(builder);
  }
}
