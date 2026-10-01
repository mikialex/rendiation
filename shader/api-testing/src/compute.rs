use crate::graphics::*;
use crate::harness::*;

/// the default workgroup size is (256, 1, 1)
#[test]
fn default_workgroup_size() {
  let module = build_compute(|builder| keep(builder.global_invocation_id()));
  validate(&module);
  assert_eq!(module.entry_points[0].workgroup_size, [256, 1, 1]);
}

/// the workgroup size of 1 to 3 dimensions, the missing dimensions are 1, and the last config
/// overrides the previous one
#[test]
fn workgroup_size_config() {
  let one = build_compute(|builder| {
    builder.config_work_group_size(64);
  });
  let two = build_compute(|builder| {
    builder.config_work_group_size((8, 4));
  });
  let three = build_compute(|builder| {
    builder.config_work_group_size((1, 1, 1));
    builder.config_work_group_size((4, 2, 8));
  });
  for (module, expect) in [(one, [64, 1, 1]), (two, [8, 4, 1]), (three, [4, 2, 8])] {
    validate(&module);
    assert_eq!(module.entry_points[0].workgroup_size, expect);
  }
}

/// the compute built-in inputs, the subgroup ones are only created when used
#[test]
fn compute_builtin_inputs() {
  let module = build_compute(|builder| {
    keep(builder.global_invocation_id() + builder.local_invocation_id());
    keep(builder.workgroup_id() + builder.workgroup_count());
    keep(builder.local_invocation_index() + builder.subgroup_size());
    keep(builder.subgroup_invocation_id() + builder.subgroup_id());
    keep(builder.num_subgroups() + builder.subgroup_size());
  });
  validate(&module);

  use naga::BuiltIn::*;
  assert_eq!(
    builtin_arguments(&module),
    [
      GlobalInvocationId,
      LocalInvocationId,
      LocalInvocationIndex,
      WorkGroupId,
      NumWorkGroups,
      SubgroupSize,
      SubgroupInvocationId,
      SubgroupId,
      NumSubgroups,
    ]
  );
  let entry = &module.entry_points[0];
  assert_eq!(entry.stage, naga::ShaderStage::Compute);
  assert!(entry.function.result.is_none());
  assert_eq!(entry.early_depth_test, None);
}
