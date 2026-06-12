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
      invocation.extra_register(builder);
    })
  }
}

fn prepare_gpu_sub_list_out_ranges(sub_list_infos: &[SubListHostInfo]) -> (Vec<Vec2<u32>>, u32) {
  let sub_count = sub_list_infos.len();
  let mut offset = 0u32;
  let mut ranges = Vec::with_capacity(sub_count);
  for info in sub_list_infos.iter() {
    ranges.push(Vec2::new(offset, info.capacity));
    offset += info.capacity;
  }
  (ranges, offset)
}

pub fn use_and_create_default_indirect_draw_provider(
  list: &DeviceDrawList,
  draw_command_builder: DrawCommandBuilder,
  cx: &mut DeviceParallelComputeCtx,
  enable_midc_downgrade: bool,
) -> Vec<Box<dyn IndirectDrawProvider>> {
  let results = match draw_command_builder {
    DrawCommandBuilder::Indexed(generator) => cx.scope(|cx| {
      let generator = IndexedDrawCommandGeneratorComponent {
        scene_models: list.clone().into_boxed(),
        generator,
      };

      let size = generator.result_size();
      let init = ZeroedArrayByArrayLength(size as usize);
      let draw_command_buffer = StorageBufferDataView::create_by_with_extra_usage(
        cx.gpu.device.as_ref(),
        StorageBufferInit::<[DrawIndexedIndirectArgsStorage]>::from(init),
        BufferUsages::INDIRECT,
      );
      let (output_ranges_host, size_all) =
        prepare_gpu_sub_list_out_ranges(&list.dispatch_info.sub_list_infos);
      assert_eq!(size_all, size);
      let output_ranges =
        create_gpu_readonly_storage(output_ranges_host.as_slice(), cx.gpu.device.as_ref());

      let dispatch_size = generator.compute_work_size(cx);
      cx.record_pass(|pass, device| {
        let mut hasher = shader_hasher_from_marker_ty!(WriteIndexDrawCommandStorageBuffer);
        generator.hash_pipeline_with_type_info(&mut hasher);
        let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
          // todo, move 256 unwrap into parallel compute trait
          builder.config_work_group_size(generator.requested_workgroup_size().unwrap_or(256));
          let generator = generator.build_shader(&mut builder);
          let output_ranges = builder.bind_by(&output_ranges);
          let input_ranges = builder.bind_by(&list.dispatch_info.sub_list_ranges);
          let write_target = builder.bind_by(&draw_command_buffer);

          let ((cmd, list_index), valid) =
            generator.invocation_logic(builder.global_invocation_id());
          if_by(valid, || {
            let range_write_offset = output_ranges.index(list_index).load().x();
            let range_base_offset = input_ranges.index(list_index).load().z();
            let range_relative_index = builder.global_invocation_id().x() - range_base_offset;
            let write_index = range_relative_index + range_write_offset;
            write_target.index(write_index).store(cmd);
          });

          builder
        });

        BindingBuilder::default()
          .with_fn(|b| generator.bind_input(b))
          .with_bind(&output_ranges)
          .with_bind(&list.dispatch_info.sub_list_ranges)
          .with_bind(&draw_command_buffer)
          .setup_compute_pass(pass, device, &pipeline);

        pass.dispatch_workgroups_indirect_by_buffer_resource_view(&dispatch_size.0);
      });

      let command_pool_ro = draw_command_buffer.into_readonly_view();
      let counts_views = list.create_indirect_count_views();
      let cmd_views = create_pool_views(command_pool_ro.clone(), &output_ranges_host);

      if enable_midc_downgrade {
        let command_pool = StorageDrawCommands::Indexed(command_pool_ro.into());
        let midc_input = rendiation_webgpu_midc_downgrade::MIDCListPoolInput {
          command_pool,
          list_info: list.dispatch_info.clone(),
        };
        let downgraded =
          rendiation_webgpu_midc_downgrade::downgrade_multi_indirect_draw_count_list_pool(
            midc_input, cx,
          );
        downgraded
          .into_iter()
          .zip(counts_views.into_iter())
          .zip(cmd_views.into_iter())
          .map(|(((helper, cmd), draw_count), cmd_view)| {
            let cmd_buf = StorageBufferReadonlyDataView::try_from_raw(cmd_view).unwrap();
            let internal = MultiIndirectDrawBatch {
              draw_command_buffer: StorageDrawCommands::Indexed(cmd_buf.into()),
              draw_count,
            };
            Box::new(MIDCDowngradeBatch {
              helper,
              cmd,
              internal,
            }) as Box<dyn IndirectDrawProvider>
          })
          .collect()
      } else {
        counts_views
          .into_iter()
          .zip(cmd_views.into_iter())
          .map(|(draw_count, cmd)| {
            let cmd = StorageBufferReadonlyDataView::try_from_raw(cmd).unwrap();
            let provider = MultiIndirectDrawBatch {
              draw_command_buffer: StorageDrawCommands::Indexed(cmd.into()),
              draw_count,
            };
            Box::new(provider) as Box<dyn IndirectDrawProvider>
          })
          .collect()
      }
    }),
    DrawCommandBuilder::NoneIndexed(generator) => cx.scope(|cx| {
      let generator = DrawCommandGeneratorComponent {
        scene_models: list.clone().into_boxed(),
        generator,
      };

      let size = generator.result_size();
      let init = ZeroedArrayByArrayLength(size as usize);
      let draw_command_buffer = StorageBufferDataView::create_by_with_extra_usage(
        cx.gpu.device.as_ref(),
        StorageBufferInit::<[DrawIndirectArgsStorage]>::from(init),
        BufferUsages::INDIRECT,
      );

      let (output_ranges_host, size_all) =
        prepare_gpu_sub_list_out_ranges(&list.dispatch_info.sub_list_infos);
      assert_eq!(size_all, size);
      let output_ranges =
        create_gpu_readonly_storage(output_ranges_host.as_slice(), cx.gpu.device.as_ref());

      let dispatch_size = generator.compute_work_size(cx);
      cx.record_pass(|pass, device| {
        let mut hasher = shader_hasher_from_marker_ty!(WriteDrawCommandStorageBuffer);
        generator.hash_pipeline_with_type_info(&mut hasher);
        let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
          // todo, move 256 unwrap into parallel compute trait
          builder.config_work_group_size(generator.requested_workgroup_size().unwrap_or(256));
          let generator = generator.build_shader(&mut builder);
          let output_ranges = builder.bind_by(&output_ranges);
          let input_ranges = builder.bind_by(&list.dispatch_info.sub_list_ranges);
          let write_target = builder.bind_by(&draw_command_buffer);

          let ((cmd, list_index), valid) =
            generator.invocation_logic(builder.global_invocation_id());
          if_by(valid, || {
            let range_write_offset = output_ranges.index(list_index).load().x();
            let range_base_offset = input_ranges.index(list_index).load().z();

            let range_relative_index = builder.global_invocation_id().x() - range_base_offset;
            let write_index = range_relative_index + range_write_offset;
            write_target.index(write_index).store(cmd);
          });

          builder
        });

        BindingBuilder::default()
          .with_fn(|b| generator.bind_input(b))
          .with_bind(&output_ranges)
          .with_bind(&list.dispatch_info.sub_list_ranges)
          .with_bind(&draw_command_buffer)
          .setup_compute_pass(pass, device, &pipeline);

        pass.dispatch_workgroups_indirect_by_buffer_resource_view(&dispatch_size.0);
      });

      let command_pool_ro = draw_command_buffer.into_readonly_view();
      let counts_views = list.create_indirect_count_views();
      let cmd_views = create_pool_views(command_pool_ro.clone(), &output_ranges_host);

      if enable_midc_downgrade {
        let command_pool = StorageDrawCommands::NoneIndexed(command_pool_ro.into());
        let midc_input = rendiation_webgpu_midc_downgrade::MIDCListPoolInput {
          command_pool,
          list_info: list.dispatch_info.clone(),
        };
        let downgraded =
          rendiation_webgpu_midc_downgrade::downgrade_multi_indirect_draw_count_list_pool(
            midc_input, cx,
          );
        downgraded
          .into_iter()
          .zip(counts_views.into_iter())
          .zip(cmd_views.into_iter())
          .map(|(((helper, cmd), draw_count), cmd_view)| {
            let cmd_buf = StorageBufferReadonlyDataView::try_from_raw(cmd_view).unwrap();
            let internal = MultiIndirectDrawBatch {
              draw_command_buffer: StorageDrawCommands::NoneIndexed(cmd_buf.into()),
              draw_count,
            };
            Box::new(MIDCDowngradeBatch {
              helper,
              cmd,
              internal,
            }) as Box<dyn IndirectDrawProvider>
          })
          .collect()
      } else {
        counts_views
          .into_iter()
          .zip(cmd_views.into_iter())
          .map(|(draw_count, cmd)| {
            let cmd = StorageBufferReadonlyDataView::try_from_raw(cmd).unwrap();
            let provider = MultiIndirectDrawBatch {
              draw_command_buffer: StorageDrawCommands::NoneIndexed(cmd.into()),
              draw_count,
            };
            Box::new(provider) as Box<dyn IndirectDrawProvider>
          })
          .collect()
      }
    }),
  };

  results
}

/// the pool size is the sum of all sub-lists capacity, but the the sub list if only
/// reference part of the address space. so the offset should be compted
/// based on the sub-lists offset
fn create_pool_views<T: Std430>(
  pool: StorageBufferReadonlyDataView<[T]>,
  offset_count: &[Vec2<u32>],
) -> Vec<GPUBufferResourceView> {
  let mut cmd_views = Vec::with_capacity(offset_count.len());
  for offset_count in offset_count {
    let item_size = std::mem::size_of::<T>() as u64;
    let view = pool.gpu.resource.create_view(GPUBufferViewRange {
      offset: offset_count.x() as u64 * item_size,
      size: std::num::NonZeroU64::new(offset_count.y() as u64 * item_size).into(),
    });
    cmd_views.push(view);
  }
  cmd_views
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
      indexed: matches!(&self.draw_command_buffer, StorageDrawCommands::Indexed(_)),
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
