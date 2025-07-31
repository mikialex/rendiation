use rendiation_webgpu_midc_downgrade::*;

use crate::*;

/// if the T using VertexInstanceIndex as draw id, this function can be used
pub fn into_maybe_downgrade_batch_assume_standard_midc_style<T: IndirectDrawProvider + 'static>(
  batch: T,
  cx: &mut DeviceParallelComputeCtx,
) -> Box<dyn IndirectDrawProvider> {
  if require_midc_downgrade(&cx.gpu.info) {
    let (helper, cmd) = rendiation_webgpu_midc_downgrade::downgrade_multi_indirect_draw_count(
      batch.draw_command(),
      cx,
    );
    Box::new(MIDCDowngradeBatch {
      helper,
      cmd,
      internal: batch,
    })
  } else {
    Box::new(batch)
  }
}

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
    let vertex_index = builder.query::<VertexIndex>();

    let MultiDrawDowngradeVertexInfo {
      sub_draw_command_idx: _,
      vertex_index_inside_sub_draw,
      base_vertex_or_index_offset_for_sub_draw,
      base_instance,
    } = self.helper.get_current_vertex_draw_info(vertex_index);

    builder.register::<VertexIndexForMIDCDowngrade>(
      vertex_index_inside_sub_draw + base_vertex_or_index_offset_for_sub_draw,
    );

    builder.register::<VertexInstanceIndex>(base_instance);

    base_instance
  }

  fn extra_register(&self, builder: &mut ShaderVertexBuilder) {
    self.base.extra_register(builder);
  }
}
