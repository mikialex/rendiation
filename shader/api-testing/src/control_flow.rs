use rendiation_shader_api::*;

use crate::harness::*;

/// atomic compare exchange
#[test]
fn atomic() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let atomic = builder.define_workgroup_shared_var::<DeviceAtomic<u32>>();
    keep(atomic.atomic_add(u) + atomic.atomic_max(u));
    let (old, exchanged) = atomic.atomic_compare_exchange_weak(val(0), u);
    keep(old);
    keep(exchanged);
    workgroup_barrier();
    storage_barrier();
  });
}

/// control flow
#[test]
fn control_flow() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    shader_assert(u.less_than(1024));
    switch_by(u)
      .case(0, || {})
      .case(1, || {})
      .end_with_default(|| {});
    loop_by(|cx| {
      if_by(u.equals(0), || cx.do_break()).else_by(|| cx.do_continue());
    });
  });
}

/// the switch case selector values must be distinct
#[test]
#[should_panic(expected = "switch case selector values must be distinct")]
fn switch_duplicate_case() {
  build_compute(|builder| {
    switch_by(runtime_values(builder).u)
      .case(1, || {})
      .case(1, || {})
      .end_with_default(|| {});
  });
}
