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

/// break and continue the loop in nested if, switch and inner loop
#[test]
fn nested_loop_control() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    loop_by(|outer| {
      if_by(u.equals(0), || outer.do_break());
      switch_by(u)
        .case(0, || {})
        // continue is not affected by the switch in between
        .case(1, || outer.do_continue())
        .end_with_default(|| {});
      loop_by(|inner| {
        if_by(u.equals(1), || inner.do_continue());
        inner.do_break();
      });
      outer.do_break();
    });
  });
}

/// WGSL can not break the outer loop
#[test]
#[should_panic(expected = "break an outer loop inside the inner loop is not supported")]
fn break_outer_loop() {
  build_compute(|_| {
    loop_by(|outer| {
      loop_by(|_| outer.do_break());
    });
  });
}

/// WGSL can not continue the outer loop
#[test]
#[should_panic(expected = "continue an outer loop inside the inner loop is not supported")]
fn continue_outer_loop() {
  build_compute(|_| {
    loop_by(|outer| {
      loop_by(|_| outer.do_continue());
    });
  });
}

/// the WGSL break inside the switch case only exits the switch
#[test]
#[should_panic(expected = "break a loop inside the switch case is not supported")]
fn break_loop_in_switch() {
  build_compute(|builder| {
    let u = runtime_values(builder).u;
    loop_by(|cx| {
      switch_by(u)
        .case(0, || cx.do_break())
        .end_with_default(|| {});
    });
  });
}

/// the loop outside of a function can not be targeted inside the function
#[test]
#[should_panic(expected = "break a loop outside of the loop")]
fn break_loop_in_function() {
  build_compute(|_| {
    loop_by(|cx| {
      get_shader_fn::<f32>("break_loop_in_function".to_string()).or_define(|builder| {
        cx.do_break();
        builder.do_return(val(0.));
      });
    });
  });
}
