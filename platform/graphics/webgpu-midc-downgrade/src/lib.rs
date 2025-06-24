use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

/// downgrade midc into single none-index indirect draw with helper access data.
///
/// the sub draw command not support instance count > 1
///
/// currently only support index draw
pub fn downgrade_multi_indirect_draw_count(
  draw: DrawCommand,
  cx: &mut DeviceParallelComputeCtx,
) -> (DowngradeMultiIndirectDrawCountHelper, DrawCommand) {
  if let DrawCommand::MultiIndirectCount {
    indexed,
    indirect_buffer,
    indirect_count,
    ..
  } = draw
  {
    let draw_commands = if indexed {
      DrawCommands::Indexed(StorageBufferReadonlyDataView::try_from_raw(indirect_buffer).unwrap())
    } else {
      DrawCommands::NoneIndexed(
        StorageBufferReadonlyDataView::try_from_raw(indirect_buffer).unwrap(),
      )
    };
    assert!(draw_commands.cmd_count() > 0);
    let draw_count = StorageBufferReadonlyDataView::try_from_raw(indirect_count).unwrap();

    let DeviceMaterializeResult {
      buffer: sub_draw_range_start_prefix_sum,
      ..
    } = MultiIndirectCountDowngradeSource {
      indirect_buffer: draw_commands.clone(),
      indirect_count: draw_count.clone(),
    }
    .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(1024, 1024)
    .make_global_scan_exclusive::<AdditionMonoid<u32>>()
    .materialize_storage_buffer(cx);

    // because we using exclusive scan
    assert_eq!(
      sub_draw_range_start_prefix_sum.item_count(),
      draw_commands.cmd_count() + 1
    );

    let indirect_buffer = StorageBufferDataView::create_by_with_extra_usage(
      &cx.gpu.device,
      StorageBufferSizedZeroed::<DrawIndirect>::default().into(),
      BufferUsages::INDIRECT,
    );

    cx.record_pass(|pass, device| {
      let hasher = shader_hasher_from_marker_ty!(PrepareIndirectDraw);
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder.config_work_group_size(1);
        let indirect_buffer = builder.bind_by(&indirect_buffer);
        let draw_count = builder.bind_by(&draw_count).load();
        let prefix_buffer = builder.bind_by(&sub_draw_range_start_prefix_sum);

        let vertex_count_all = prefix_buffer.index(draw_count).load();

        let draw_dispatch = ENode::<DrawIndirect> {
          vertex_count: vertex_count_all,
          instance_count: val(1),
          first_vertex: val(0),
          first_instance: val(0),
        }
        .construct();

        indirect_buffer.store(draw_dispatch);

        builder
      });

      BindingBuilder::default()
        .with_bind(&indirect_buffer)
        .with_bind(&draw_count)
        .with_bind(&sub_draw_range_start_prefix_sum)
        .setup_compute_pass(pass, device, &pipeline);

      pass.dispatch_workgroups(1, 1, 1);
    });

    (
      DowngradeMultiIndirectDrawCountHelper {
        sub_draw_range_start_prefix_sum,
        draw_commands,
      },
      DrawCommand::Indirect {
        indirect_buffer: indirect_buffer.gpu,
        indexed: false,
      },
    )
  } else {
    panic!("expect midc draw command");
  }
}

pub struct DowngradeMultiIndirectDrawCountHelper {
  sub_draw_range_start_prefix_sum: StorageBufferReadonlyDataView<[u32]>,
  draw_commands: DrawCommands,
}

impl DowngradeMultiIndirectDrawCountHelper {
  pub fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> DowngradeMultiIndirectDrawCountHelperInvocation {
    DowngradeMultiIndirectDrawCountHelperInvocation {
      sub_draw_range_start_prefix_sum: cx.bind_by(&self.sub_draw_range_start_prefix_sum),
      draw_commands: self.draw_commands.build(cx),
    }
  }
  pub fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.sub_draw_range_start_prefix_sum);
    self.draw_commands.bind(builder);
  }
}

pub struct DowngradeMultiIndirectDrawCountHelperInvocation {
  sub_draw_range_start_prefix_sum: ShaderReadonlyPtrOf<[u32]>,
  draw_commands: DrawCommandsInvocation,
}

pub struct MultiDrawDowngradeVertexInfo {
  pub sub_draw_command_idx: Node<u32>,
  pub vertex_index_inside_sub_draw: Node<u32>,
  pub base_vertex_or_index_offset_for_sub_draw: Node<u32>,
  pub base_instance: Node<u32>,
}

impl DowngradeMultiIndirectDrawCountHelperInvocation {
  pub fn get_current_vertex_draw_info(&self, vertex_id: Node<u32>) -> MultiDrawDowngradeVertexInfo {
    // binary search for current draw command
    let start = val(0_u32).make_local_var();
    let end = (self.sub_draw_range_start_prefix_sum.array_length() - val(2)).make_local_var();

    loop_by(|cx| {
      if_by(start.load().greater_equal_than(end.load()), || {
        cx.do_break()
      });

      let mid = (start.load() + end.load()) / val(2);
      let test = self
        .sub_draw_range_start_prefix_sum
        .index(mid + val(1))
        .load();
      if_by(test.less_equal_than(vertex_id), || {
        start.store(mid + val(1)); // in [mid+ 1, end]
      })
      .else_by(|| {
        end.store(mid); // in [start, mid]
      });
    });

    let index = start.load();
    let draw_base_offset = self.sub_draw_range_start_prefix_sum.index(index).load();
    let draw_inner_offset = vertex_id - draw_base_offset;

    let (offset, base_instance) = match &self.draw_commands {
      DrawCommandsInvocation::Indexed(cmds) => {
        let draw_cmd = cmds.index(index);
        let offset = draw_cmd.base_index().load();
        let base_instance = draw_cmd.base_instance().load();
        (offset, base_instance)
      }
      DrawCommandsInvocation::NoneIndexed(cmds) => {
        let draw_cmd = cmds.index(index);
        let offset = draw_cmd.first_vertex().load();
        let base_instance = draw_cmd.first_instance().load();
        (offset, base_instance)
      }
    };

    MultiDrawDowngradeVertexInfo {
      sub_draw_command_idx: index,
      vertex_index_inside_sub_draw: draw_inner_offset,
      base_vertex_or_index_offset_for_sub_draw: offset,
      base_instance,
    }
  }
}

#[derive(Clone)]
struct MultiIndirectCountDowngradeSource {
  indirect_buffer: DrawCommands,
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
    self.indirect_buffer.cmd_count()
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
      indirect_buffer: DrawCommandsInvocation,
      indirect_count: ShaderReadonlyPtrOf<u32>,
    }

    impl DeviceInvocation<Node<u32>> for MultiIndirectCountDowngradeSourceInvocation {
      fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
        let idx = logic_global_id.x();
        let r = idx.less_than(self.indirect_buffer.array_length());
        let result = r.select_branched(|| self.indirect_buffer.vertex_count(idx), zeroed_val);
        (result, r)
      }

      fn invocation_size(&self) -> Node<Vec3<u32>> {
        (self.indirect_count.load(), val(0), val(0)).into()
      }
    }

    Box::new(MultiIndirectCountDowngradeSourceInvocation {
      indirect_buffer: self.indirect_buffer.build(&mut builder.bindgroups),
      indirect_count: builder.bind_by(&self.indirect_count),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.indirect_buffer.bind(builder);
    builder.bind(&self.indirect_count);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
}

#[derive(Clone)]
enum DrawCommands {
  Indexed(StorageBufferReadonlyDataView<[DrawIndexedIndirect]>),
  NoneIndexed(StorageBufferReadonlyDataView<[DrawIndirect]>),
}

impl DrawCommands {
  pub fn cmd_count(&self) -> u32 {
    match self {
      DrawCommands::Indexed(v) => v.item_count(),
      DrawCommands::NoneIndexed(v) => v.item_count(),
    }
  }

  pub fn bind(&self, builder: &mut BindingBuilder) {
    match self {
      DrawCommands::Indexed(v) => builder.bind(v),
      DrawCommands::NoneIndexed(v) => builder.bind(v),
    };
  }

  pub fn build(&self, cx: &mut ShaderBindGroupBuilder) -> DrawCommandsInvocation {
    match self {
      DrawCommands::Indexed(v) => DrawCommandsInvocation::Indexed(cx.bind_by(v)),
      DrawCommands::NoneIndexed(v) => DrawCommandsInvocation::NoneIndexed(cx.bind_by(v)),
    }
  }
}

enum DrawCommandsInvocation {
  Indexed(ShaderReadonlyPtrOf<[DrawIndexedIndirect]>),
  NoneIndexed(ShaderReadonlyPtrOf<[DrawIndirect]>),
}

impl DrawCommandsInvocation {
  pub fn array_length(&self) -> Node<u32> {
    match self {
      DrawCommandsInvocation::Indexed(v) => v.array_length(),
      DrawCommandsInvocation::NoneIndexed(v) => v.array_length(),
    }
  }
  pub fn vertex_count(&self, idx: Node<u32>) -> Node<u32> {
    match self {
      DrawCommandsInvocation::Indexed(v) => v.index(idx).vertex_count().load(),
      DrawCommandsInvocation::NoneIndexed(v) => v.index(idx).vertex_count().load(),
    }
  }
}
