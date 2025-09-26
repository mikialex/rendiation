use crate::*;

#[derive(Clone)]
pub enum HostDrawCommands {
  Indexed(Vec<DrawIndexedIndirectArgsStorage>),
  NoneIndexed(Vec<DrawIndirectArgsStorage>),
}

/// downgrade midc into single none-index indirect draw with helper access data.
///
/// the sub draw command not support instance count > 1
pub fn downgrade_multi_indirect_draw_count_host_driven(
  draw: HostDrawCommands,
  gpu: &GPU,
) -> (DowngradeMultiIndirectDrawCountHelper, DrawCommand) {
  let alloc = DefaultStorageAllocator;

  let mut sub_draw_range_start_prefix_sum = match &draw {
    HostDrawCommands::Indexed(cmds) => cmds
      .iter()
      .map(|v| v.vertex_count)
      .scan(0, |state, count| {
        let current_prefix_sum = *state;
        *state += count;
        Some(current_prefix_sum)
      })
      .collect::<Vec<_>>(),
    HostDrawCommands::NoneIndexed(cmds) => cmds
      .iter()
      .map(|v| v.vertex_count)
      .scan(0, |state, count| {
        let current_prefix_sum = *state;
        *state += count;
        Some(current_prefix_sum)
      })
      .collect::<Vec<_>>(),
  };

  let last = match &draw {
    HostDrawCommands::Indexed(cmds) => cmds.last().map(|v| v.vertex_count).unwrap_or(0),
    HostDrawCommands::NoneIndexed(cmds) => cmds.last().map(|v| v.vertex_count).unwrap_or(0),
  } + sub_draw_range_start_prefix_sum.last().copied().unwrap_or(0);

  sub_draw_range_start_prefix_sum.push(last);

  let draw_call_sum = sub_draw_range_start_prefix_sum.last().copied().unwrap_or(0);

  let cmd = DrawCommand::Array {
    vertices: 0..draw_call_sum,
    instances: 0..1,
  };

  let sub_draw_range_start_prefix_sum = alloc.allocate_readonly_init(
    sub_draw_range_start_prefix_sum.as_slice(),
    gpu,
    "draw cmd count prefix sum".into(),
  );

  let draw_commands = match draw {
    HostDrawCommands::Indexed(args) => {
      let cmds = alloc.allocate_readonly_init(args.as_slice(), gpu, "draw commands".into());
      StorageDrawCommands::Indexed(cmds)
    }
    HostDrawCommands::NoneIndexed(args) => {
      let cmds = alloc.allocate_readonly_init(args.as_slice(), gpu, "draw commands".into());
      StorageDrawCommands::NoneIndexed(cmds)
    }
  };

  let helper = DowngradeMultiIndirectDrawCountHelper {
    sub_draw_range_start_prefix_sum,
    draw_commands,
  };

  (helper, cmd)
}
