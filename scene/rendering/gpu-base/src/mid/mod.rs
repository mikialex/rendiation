use crate::*;

mod draw_cmd;
pub use draw_cmd::*;

mod midc_downgrade;
pub use midc_downgrade::*;

#[derive(Clone)]
pub enum DrawCommandBuilder {
  Indexed(Box<dyn TypedDrawCommandBuilder<DrawIndexedIndirectArgsStorage>>),
  NoneIndexed(Box<dyn TypedDrawCommandBuilder<DrawIndirectArgsStorage>>),
}

impl DrawCommandBuilder {
  pub fn draw_command_host_access(
    &self,
    id: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommand> {
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
      builder.register::<RootLogicalRenderEntityId>(scene_model_id);
      invocation.extra_register(builder);
    })
  }
}

/// Generate the draw commands of each sub list and create one draw provider for each sub list.
///
/// The draw commands of the sub lists are placed in one pool by [compact_capacity_ranges], so
/// the command views of each sub list satisfy the storage buffer offset alignment if the sub
/// list capacity is aligned.
pub fn use_and_create_default_indirect_draw_provider(
  list: &DeviceDrawList,
  draw_command_builder: DrawCommandBuilder,
  cx: &mut DeviceParallelComputeCtx,
  enable_midc_downgrade: bool,
) -> Vec<Box<dyn IndirectDrawProvider>> {
  cx.next_scope_index();

  let (cmd_layout, cmd_pool_size) =
    compact_capacity_ranges(&list.dispatch_info.host_capacity_ranges);
  let cmd_layout = list.dispatch_info.use_relayout(cx, &cmd_layout);

  let command_pool = match draw_command_builder {
    DrawCommandBuilder::Indexed(generator) => {
      cx.scope(|cx| use_draw_command_pool(cx, list, &cmd_layout, cmd_pool_size, generator))
    }
    DrawCommandBuilder::NoneIndexed(generator) => {
      cx.scope(|cx| use_draw_command_pool(cx, list, &cmd_layout, cmd_pool_size, generator))
    }
  };

  if enable_midc_downgrade {
    return cx.scope(|cx| {
      let midc_input = rendiation_webgpu_midc_downgrade::MIDCListPoolInput {
        command_pool,
        list_info: cmd_layout,
      };

      rendiation_webgpu_midc_downgrade::use_downgrade_multi_indirect_draw_count_list_pool(
        midc_input, cx,
      )
      .into_iter()
      .map(|(helper, cmd)| {
        Box::new(MIDCDowngradeBatch { helper, cmd }) as Box<dyn IndirectDrawProvider>
      })
      .collect()
    });
  }

  let align = cx
    .gpu
    .info
    .supported_limits
    .min_storage_buffer_offset_alignment as u64;
  let command_size = command_pool.command_byte_size();

  let draw_counts = cmd_layout.device_ranges.create_indirect_count_views();
  draw_counts
    .into_iter()
    .zip(&cmd_layout.host_capacity_ranges)
    .map(|(draw_count, range)| {
      assert!((range.offset as u64 * command_size).is_multiple_of(align));
      Box::new(MultiIndirectDrawBatch {
        draw_command_buffer: command_pool.sub_view(range.offset, range.capacity),
        draw_count,
      }) as Box<dyn IndirectDrawProvider>
    })
    .collect()
}

/// Write the draw command of each element of the list into the pool placed by the cmd_layout.
fn use_draw_command_pool<C: DrawCommandStorage>(
  cx: &mut DeviceParallelComputeCtx,
  list: &DeviceDrawList,
  cmd_layout: &MultiRangeDispatchInfo,
  cmd_pool_size: u32,
  generator: Box<dyn TypedDrawCommandBuilder<C>>,
) -> StorageDrawCommands {
  let generator = DrawCommandGeneratorComponent {
    scene_models: list.clone().into_boxed(),
    generator,
  };

  let draw_command_buffer = cx.use_rw_storage_buffer_array_impl::<C>(
    cmd_pool_size as usize,
    "draw command buffer",
    BufferUsages::INDIRECT,
  );

  let cmd_ranges = &cmd_layout.device_ranges.sub_list_ranges;
  let dispatch_size = generator.compute_work_size(cx);
  cx.record_pass(|pass, device| {
    let mut hasher = shader_hasher_from_marker_ty!(WriteDrawCommandStorageBuffer);
    generator.hash_pipeline_with_type_info(&mut hasher);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      // todo, move 256 unwrap into parallel compute trait
      builder.config_work_group_size(generator.requested_workgroup_size().unwrap_or(256));
      let generator = generator.build_shader(&mut builder);
      let cmd_ranges = builder.bind_by(cmd_ranges);
      let draw_command_buffer = builder.bind_by(&draw_command_buffer);

      let global_id = builder.global_invocation_id().x();
      let ((cmd, list_index), valid) = generator.invocation_logic(builder.global_invocation_id());
      if_by(valid, || {
        // the cmd layout has the same count prefix sum as the list, only the offset differs
        let range = cmd_ranges.index(list_index).load().expand();
        let write_index = global_id - range.count_prefix_sum + range.offset;
        draw_command_buffer.index(write_index).store(cmd);
      });

      builder
    });

    BindingBuilder::default()
      .with_fn(|b| generator.bind_input(b))
      .with_bind(cmd_ranges)
      .with_bind(&draw_command_buffer)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups_indirect_by_buffer_resource_view(&dispatch_size.0);
  });

  C::into_storage_draw_commands(draw_command_buffer.into_readonly_view())
}

struct MultiIndirectDrawBatch {
  draw_command_buffer: StorageDrawCommands,
  draw_count: GPUBufferResourceView,
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
      indexed: self.draw_command_buffer.is_index(),
      indirect_buffer: self.draw_command_buffer.indirect_buffer(),
      indirect_count: self.draw_count.clone(),
      max_count: self.draw_command_buffer.cmd_capacity_count(),
    }
  }
}

impl ShaderPassBuilder for MultiIndirectDrawBatch {}
impl ShaderHashProvider for MultiIndirectDrawBatch {
  shader_hash_type_id! {}
}
