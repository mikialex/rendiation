use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

/// downgrade midc into single indirect draw with helper access data.
///
/// currently only support index draw without any instance sub draw
pub fn downgrade_multi_indirect_draw_count(
  draw: DrawCommand,
  frame_ctx: &mut FrameCtx,
) -> (DowngradeMultiIndirectDrawCountHelper, DrawCommand) {
  if let DrawCommand::MultiIndirectCount {
    indexed,
    indirect_buffer,
    indirect_count,
    ..
  } = draw
  {
    assert!(indexed);

    let draw_commands = StorageBufferReadonlyDataView::try_from_raw(indirect_buffer).unwrap();
    let draw_count = StorageBufferReadonlyDataView::try_from_raw(indirect_count).unwrap();

    let (sub_draw_range_start_prefix_sum, indirect_buffer) =
      frame_ctx.access_parallel_compute(|cx| {
        let DeviceMaterializeResult { buffer, .. } = MultiIndirectCountDowngradeSource {
          indirect_buffer: draw_commands.clone(),
          indirect_count: draw_count.clone(),
        }
        .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(1024, 1024)
        .materialize_storage_buffer(cx);

        let indirect_buffer = StorageBufferDataView::create_by_with_extra_usage(
          &frame_ctx.gpu.device,
          StorageBufferSizedZeroed::<DrawIndexedIndirect>::default().into(),
          BufferUsages::INDIRECT,
        );

        cx.record_pass(|pass, device| {
          let hasher = shader_hasher_from_marker_ty!(PrepareIndirectDraw);
          let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
            builder.config_work_group_size(1);
            let indirect_buffer = builder.bind_by(&indirect_buffer);
            let draw_count = builder.bind_by(&draw_count).load();
            let prefix_buffer = builder.bind_by(&buffer);

            let vertex_count_all = prefix_buffer.index(draw_count).load();

            let draw_dispatch = ENode::<DrawIndexedIndirect> {
              vertex_count: vertex_count_all,
              instance_count: val(0),
              base_index: val(0),
              vertex_offset: val(0),
              base_instance: val(0),
            }
            .construct();

            indirect_buffer.store(draw_dispatch);

            builder
          });

          BindingBuilder::default()
            .with_bind(&indirect_buffer)
            .with_bind(&draw_count)
            .with_bind(&buffer)
            .setup_compute_pass(pass, device, &pipeline);

          pass.dispatch_workgroups(1, 1, 1);
        });

        (buffer, indirect_buffer)
      });

    (
      DowngradeMultiIndirectDrawCountHelper {
        sub_draw_range_start_prefix_sum,
        draw_commands,
      },
      DrawCommand::Indirect {
        indirect_buffer: indirect_buffer.gpu,
        indexed: true,
      },
    )
  } else {
    panic!("expect midc draw command");
  }
}

pub struct DowngradeMultiIndirectDrawCountHelper {
  sub_draw_range_start_prefix_sum: StorageBufferReadonlyDataView<[u32]>,
  draw_commands: StorageBufferReadonlyDataView<[DrawIndexedIndirect]>,
}

impl DowngradeMultiIndirectDrawCountHelper {
  pub fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> DowngradeMultiIndirectDrawCountHelperInvocation {
    DowngradeMultiIndirectDrawCountHelperInvocation {
      sub_draw_range_start_prefix_sum: cx.bind_by(&self.sub_draw_range_start_prefix_sum),
      draw_commands: cx.bind_by(&self.draw_commands),
    }
  }
  pub fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.sub_draw_range_start_prefix_sum);
    builder.bind(&self.draw_commands);
  }
}

pub struct DowngradeMultiIndirectDrawCountHelperInvocation {
  sub_draw_range_start_prefix_sum: ShaderReadonlyPtrOf<[u32]>,
  draw_commands: ShaderReadonlyPtrOf<[DrawIndexedIndirect]>,
}

impl DowngradeMultiIndirectDrawCountHelperInvocation {
  pub fn get_current_vertex_draw_command_index_and_info(
    &self,
    vertex_id: Node<u32>,
  ) -> (Node<u32>, Node<DrawIndexedIndirect>) {
    // binary search for current draw command
    let start = val(0_u32).make_local_var();
    let end = (self.sub_draw_range_start_prefix_sum.array_length() - val(1)).make_local_var();

    loop_by(|cx| {
      if_by(start.load().equals(end.load()), || cx.do_break());

      let mid = (start.load() + end.load()) / val(2);
      let test = self.sub_draw_range_start_prefix_sum.index(mid).load();
      if_by(vertex_id.less_equal_than(test), || {
        end.store(mid);
      });
      if_by(vertex_id.greater_equal_than(test), || {
        start.store(mid + val(1));
      });
    });

    let index = start.load();
    let draw_command = self.draw_commands.index(index).load();
    (index, draw_command)
  }
}

#[derive(Clone)]
struct MultiIndirectCountDowngradeSource {
  indirect_buffer: StorageBufferReadonlyDataView<[DrawIndexedIndirect]>,
  indirect_count: StorageBufferReadonlyDataView<u32>,
}

impl ShaderHashProvider for MultiIndirectCountDowngradeSource {
  shader_hash_type_id! {}
}

impl DeviceParallelCompute<Node<u32>> for MultiIndirectCountDowngradeSource {
  fn execute_and_expose(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn result_size(&self) -> u32 {
    self.indirect_buffer.item_count()
  }
}
impl DeviceParallelComputeIO<u32> for MultiIndirectCountDowngradeSource {}

impl DeviceInvocationComponent<Node<u32>> for MultiIndirectCountDowngradeSource {
  fn work_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    struct MultiIndirectCountDowngradeSourceInvocation {
      indirect_buffer: ShaderReadonlyPtrOf<[DrawIndexedIndirect]>,
      indirect_count: ShaderReadonlyPtrOf<u32>,
    }

    impl DeviceInvocation<Node<u32>> for MultiIndirectCountDowngradeSourceInvocation {
      fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
        // todo, assert instance count == 1 in shader

        let idx = logic_global_id.x();
        let r = idx.less_than(self.indirect_buffer.array_length());
        let result = r.select_branched(
          || self.indirect_buffer.index(idx).vertex_count().load(),
          zeroed_val,
        );
        (result, r)
      }

      fn invocation_size(&self) -> Node<Vec3<u32>> {
        (self.indirect_count.load(), val(0), val(0)).into()
      }
    }

    Box::new(MultiIndirectCountDowngradeSourceInvocation {
      indirect_buffer: builder.bind_by(&self.indirect_buffer),
      indirect_count: builder.bind_by(&self.indirect_count),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.indirect_buffer);
    builder.bind(&self.indirect_count);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
}
