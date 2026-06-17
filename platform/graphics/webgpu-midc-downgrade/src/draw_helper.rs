use crate::*;

pub struct DowngradeMultiIndirectDrawCountHelper {
  pub(crate) sub_draw_range_start_prefix_sum: AbstractReadonlyStorageBuffer<[u32]>,
  pub(crate) draw_count: AbstractReadonlyStorageBuffer<u32>,
  pub(crate) draw_commands: StorageDrawCommands,
}

impl ShaderHashProvider for DowngradeMultiIndirectDrawCountHelper {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.draw_commands.is_index());
  }
}

impl DowngradeMultiIndirectDrawCountHelper {
  pub fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> DowngradeMultiIndirectDrawCountHelperInvocation {
    DowngradeMultiIndirectDrawCountHelperInvocation {
      sub_draw_range_start_prefix_sum: cx.bind_by(&self.sub_draw_range_start_prefix_sum),
      draw_commands: self.draw_commands.build(cx),
      real_draw_command_count: cx.bind_by(&self.draw_count).load(),
    }
  }
  pub fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.sub_draw_range_start_prefix_sum);
    self.draw_commands.bind(builder);
    builder.bind(&self.draw_count);
  }
}

pub struct DowngradeMultiIndirectDrawCountHelperInvocation {
  sub_draw_range_start_prefix_sum: ShaderReadonlyPtrOf<[u32]>,
  real_draw_command_count: Node<u32>,
  draw_commands: StorageDrawCommandsInvocation,
}

pub struct MultiDrawDowngradeVertexInfo {
  pub sub_draw_command_idx: Node<u32>,
  pub vertex_index_inside_sub_draw: Node<u32>,
  pub base_vertex_or_index_offset_for_sub_draw: Node<u32>,
  pub base_instance: Node<u32>,
}

impl DowngradeMultiIndirectDrawCountHelperInvocation {
  pub fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
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
    builder.register::<VertexIndexForMIDCDowngradeRelative>(vertex_index_inside_sub_draw);

    builder.register::<VertexInstanceIndex>(base_instance);

    base_instance
  }

  fn get_current_vertex_draw_info(&self, vertex_id: Node<u32>) -> MultiDrawDowngradeVertexInfo {
    // binary search for current draw command
    let start = val(0_u32).make_local_var();
    let end = (self.real_draw_command_count - val(1)).make_local_var();

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
      StorageDrawCommandsInvocation::Indexed(cmds) => {
        let draw_cmd = cmds.index(index);
        let offset = draw_cmd.base_index().load();
        let base_instance = draw_cmd.base_instance().load();
        (offset, base_instance)
      }
      StorageDrawCommandsInvocation::NoneIndexed(cmds) => {
        let draw_cmd = cmds.index(index);
        let offset = draw_cmd.base_vertex().load();
        let base_instance = draw_cmd.base_instance().load();
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
