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

/// the loop outside of a function can not be continued inside the function
#[test]
#[should_panic(expected = "continue a loop outside of the loop")]
fn continue_loop_in_function() {
  build_compute(|_| {
    loop_by(|cx| {
      get_shader_fn::<f32>("continue_loop_in_function".to_string()).or_define(|builder| {
        cx.do_continue();
        builder.do_return(val(0.));
      });
      cx.do_break();
    });
  });
}

/// the loop ctx moved out of the loop closure can not break after the loop is closed
#[test]
#[should_panic(expected = "break a loop outside of the loop")]
fn break_loop_after_loop_closed() {
  build_compute(|_| {
    let mut escaped = None;
    loop_by(|cx| {
      cx.do_break();
      escaped = Some(cx);
    });
    escaped.unwrap().do_break();
  });
}

/// the loop ctx moved out of the loop closure can not continue after the loop is closed
#[test]
#[should_panic(expected = "continue a loop outside of the loop")]
fn continue_loop_after_loop_closed() {
  build_compute(|_| {
    let mut escaped = None;
    loop_by(|cx| {
      cx.do_break();
      escaped = Some(cx);
    });
    escaped.unwrap().do_continue();
  });
}

/// the if inside the switch case does not change the break target, it is still the switch
#[test]
#[should_panic(expected = "break a loop inside the switch case is not supported")]
fn break_loop_in_branch_of_switch_case() {
  build_compute(|builder| {
    let u = runtime_values(builder).u;
    loop_by(|cx| {
      switch_by(u)
        .case(0, || {
          if_by(u.equals(0), || cx.do_break());
        })
        .end_with_default(|| {});
    });
  });
}

/// the switch must be ended by end_with_default, WGSL requires the default case
#[test]
#[should_panic(expected = "SwitchBuilder dropped without end_with_default")]
fn switch_without_default() {
  build_compute(|builder| {
    let u = runtime_values(builder).u;
    switch_by(u).case(0, || {}).case(1, || {});
  });
}

/// the if chain with else_if must be closed by else_by or else_over, otherwise the else scopes are
/// never closed
#[test]
#[should_panic(expected = "the if chain with else_if must be ended by else_by or else_over")]
fn else_if_without_end() {
  build_compute(|builder| {
    let u = runtime_values(builder).u;
    #[allow(unused_must_use)]
    if_by(u.equals(0), || {}).else_if(u.equals(1), || {});
  });
}

/// the backend rejects building the shader when some scope is not closed
#[test]
#[should_panic(expected = "the shader scopes are not balanced when building")]
fn build_with_unclosed_scope() {
  let mut api = rendiation_shader_backend_naga::ShaderAPINagaImpl::new(ShaderStage::Compute);
  api.push_scope();
  api.build();
}

/// all the forms of the if chain: several else_if, else_over without the final else, empty
/// branches, the chains nested in every kind of branch, in the loop and the switch case, and the
/// chain right after another if in the same block
#[test]
fn if_chains() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let r = val(0_u32).make_local_var();
    if_by(u.equals(0), || {});
    if_by(u.equals(1), || {}).else_by(|| {});
    if_by(u.equals(2), || {}).else_over();
    if_by(u.equals(2), || {})
      .else_if(u.equals(3), || {})
      .else_over();
    if_by(u.equals(4), || r.store(val(1)))
      .else_if(u.equals(5), || r.store(val(2)))
      .else_if(u.equals(6), || r.store(val(3)))
      .else_if(u.equals(7), || r.store(val(4)))
      .else_by(|| r.store(val(5)));
    if_by(u.less_than(8), || {
      if_by(u.equals(0), || r.store(val(6)))
        .else_if(u.equals(1), || r.store(val(7)))
        .else_over();
    })
    .else_if(u.less_than(16), || {
      if_by(u.equals(8), || r.store(val(8))).else_by(|| {
        if_by(u.equals(9), || r.store(val(9)))
          .else_if(u.equals(10), || {})
          .else_by(|| {});
      });
    })
    .else_by(|| {
      if_by(u.equals(16), || r.store(val(10)));
      if_by(u.equals(17), || r.store(val(11))).else_by(|| r.store(val(12)));
    });
    loop_by(|cx| {
      if_by(r.load().greater_than(100), || cx.do_break())
        .else_if(r.load().equals(50), || cx.do_continue())
        .else_by(|| r.store(r.load() + val(1)));
    });
    switch_by(u)
      .case(0, || {
        if_by(u.equals(0), || r.store(val(13)))
          .else_if(u.equals(1), || {})
          .else_over();
      })
      .end_with_default(|| {
        if_by(u.equals(2), || {}).else_by(|| r.store(val(14)));
      });
    let selected = u.less_than(3).select_branched(
      || u.equals(0).select_branched(|| val(1), || val(2)),
      || r.load(),
    );
    keep(selected);
  });
}

/// all the forms of the switch: u32 and i32 selectors with negative, extreme and unordered case
/// values, default only, nested switches, the switch in branches, and the loop, early return and
/// continue inside the cases
#[test]
fn switch_forms() {
  check_compute(|builder| {
    let RuntimeValues { u, i, .. } = runtime_values(builder);
    let r = val(0_u32).make_local_var();
    switch_by(u)
      .case(100, || r.store(val(1)))
      .case(0, || r.store(val(2)))
      .case(u32::MAX, || {})
      .case(7, || r.store(u))
      .end_with_default(|| r.store(val(3)));
    switch_by(i)
      .case(-1, || r.store(val(4)))
      .case(i32::MIN, || {})
      .case(i32::MAX, || {})
      .case(0, || r.store(val(5)))
      .case(-100, || r.store(val(6)))
      .end_with_default(|| {});
    switch_by(u).end_with_default(|| r.store(val(7)));
    switch_by(i).end_with_default(|| {});
    switch_by(u)
      .case(0, || {
        switch_by(i)
          .case(-2, || r.store(val(8)))
          .end_with_default(|| {
            switch_by(u).case(1, || {}).end_with_default(|| {});
          });
      })
      .end_with_default(|| {});
    if_by(u.equals(1), || {
      switch_by(u)
        .case(1, || r.store(val(9)))
        .end_with_default(|| {});
    })
    .else_by(|| {
      switch_by(u).end_with_default(|| r.store(val(10)));
    });
    switch_by(u)
      .case(2, || {
        // the break inside the loop nested in the case targets the nested loop
        loop_by(|cx| {
          r.store(r.load() + val(1));
          if_by(r.load().greater_than(10), || cx.do_break());
        });
      })
      .case(3, do_return)
      .end_with_default(|| {});
    loop_by(|cx| {
      switch_by(u)
        .case(0, || {
          switch_by(i)
            .case(0, || cx.do_continue())
            .end_with_default(|| {});
        })
        .end_with_default(|| cx.do_continue());
      cx.do_break();
    });
    keep(r.load());
  });
}

/// the early return in the compute entry from the nested branches, loops and switch cases, the
/// statements after the return in the same block are kept
#[test]
fn early_return_in_entry() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    if_by(u.equals(0), do_return);
    if_by(u.equals(1), || {}).else_by(|| {
      if_by(u.equals(2), do_return)
        .else_if(u.equals(3), do_return)
        .else_over();
    });
    loop_by(|cx| {
      if_by(u.equals(4), do_return);
      switch_by(u).case(5, do_return).end_with_default(|| {});
      cx.do_break();
    });
    keep(u);
    do_return();
    keep(u + val(1));
  });
}

/// the statements after break, continue and the assertion in the same block are kept and valid
#[test]
fn statements_after_jump() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    loop_by(|cx| {
      if_by(u.equals(0), || {
        cx.do_continue();
        keep(u);
      });
      cx.do_break();
      keep(u + val(1));
    });
    switch_by(u)
      .case(0, shader_unreachable)
      .end_with_default(|| {
        shader_assert(u.greater_than(0));
        keep(u);
      });
  });
}

/// Validate the shader logic, then run it on the GPU for each input value (one invocation for each
/// value) and compare with the cpu reference logic.
async fn check_gpu<I, O>(
  input: &[I],
  shader: impl Fn(Node<I>) -> Node<O> + 'static,
  cpu: impl Fn(I) -> O,
) where
  I: Std430 + ShaderSizedValueNodeType + Copy,
  O: Std430 + ShaderSizedValueNodeType + PartialEq + std::fmt::Debug,
{
  check_compute(|builder| {
    let id = builder.global_invocation_id().x();
    let v = fake_storage_buffer::<[I]>(0).index(id).load();
    fake_storage_buffer::<[O]>(1).index(id).store(shader(v));
  });
  let expect: Vec<_> = input.iter().map(|v| cpu(*v)).collect();
  assert_eq!(gpu_map(input, shader).await, expect);
}

fn range_input(end: u32) -> Vec<u32> {
  (0..end).collect()
}

/// add the value to the u32 variable
fn add(var: &ShaderPtrOf<u32>, v: impl Into<Node<u32>>) {
  var.store(var.load() + v.into());
}

/// the first matched branch of the chain is taken when the conditions overlap
#[pollster::test]
async fn if_chain_gpu() {
  check_gpu(
    &range_input(12),
    |v| {
      let r = val(0_u32).make_local_var();
      if_by(v.equals(0), || r.store(val(10)))
        .else_if(v.less_than(3), || r.store(v + val(20)))
        .else_if((v % val(2)).equals(0), || r.store(v + val(30)))
        .else_if(v.equals(5), || r.store(val(50)))
        .else_by(|| r.store(val(99)));
      r.load()
    },
    |v| match v {
      0 => 10,
      v if v < 3 => v + 20,
      v if v % 2 == 0 => v + 30,
      5 => 50,
      _ => 99,
    },
  )
  .await
}

/// the chain without the final else keeps the value when no branch is taken
#[pollster::test]
async fn if_chain_else_over_gpu() {
  check_gpu(
    &range_input(8),
    |v| {
      let r = val(7_u32).make_local_var();
      if_by(v.less_than(2), || r.store(val(1)))
        .else_if(v.less_than(4), || r.store(val(2)))
        .else_if(v.less_than(6), || r.store(val(3)))
        .else_over();
      r.load()
    },
    |v| {
      if v < 2 {
        1
      } else if v < 4 {
        2
      } else if v < 6 {
        3
      } else {
        7
      }
    },
  )
  .await
}

/// the chains nested in the if, else_if and else branches, and the chain right after a plain if in
/// the same block
#[pollster::test]
async fn if_chain_nested_gpu() {
  check_gpu(
    &range_input(14),
    |v| {
      let r = val(0_u32).make_local_var();
      if_by(v.less_than(4), || {
        if_by((v % val(2)).equals(0), || r.store(val(1))).else_by(|| r.store(val(2)));
      })
      .else_if(v.less_than(8), || {
        if_by(v.equals(4), || r.store(val(3)))
          .else_if(v.equals(5), || r.store(val(4)))
          .else_by(|| {
            if_by(v.equals(6), || r.store(val(5)))
              .else_if(v.equals(7), || r.store(val(6)))
              .else_over();
          });
      })
      .else_by(|| {
        if_by(v.less_than(10), || r.store(val(7)));
        if_by(v.equals(10), || add(&r, val(8)))
          .else_if(v.equals(11), || add(&r, val(9)))
          .else_over();
        add(&r, val(100));
      });
      r.load()
    },
    |v| match v {
      0..4 => 1 + v % 2,
      4..8 => v - 1,
      8..10 => 107,
      10 => 108,
      11 => 109,
      _ => 100,
    },
  )
  .await
}

/// the chain in the loop body, which continues and breaks the loop from its branches
#[pollster::test]
async fn if_chain_in_loop_gpu() {
  check_gpu(
    &range_input(16),
    |v| {
      let i = val(0_u32).make_local_var();
      let sum = val(0_u32).make_local_var();
      loop_by(|cx| {
        let iv = i.load();
        if_by(iv.greater_equal_than(v), || cx.do_break());
        i.store(iv + val(1));
        let m = iv % val(4);
        if_by(m.equals(0), || add(&sum, val(1)))
          .else_if(m.equals(1), || cx.do_continue())
          .else_if(m.equals(2), || {
            add(&sum, val(10));
            if_by(iv.greater_than(8), || cx.do_break());
          })
          .else_by(|| add(&sum, val(100)));
        add(&sum, val(1000));
      });
      sum.load()
    },
    |v| {
      let mut sum = 0;
      for i in 0..v {
        match i % 4 {
          0 => sum += 1,
          1 => continue,
          2 => {
            sum += 10;
            if i > 8 {
              break;
            }
          }
          _ => sum += 100,
        }
        sum += 1000;
      }
      sum
    },
  )
  .await
}

/// the nested select_branched only runs the selected branch closure
#[pollster::test]
async fn select_branched_gpu() {
  check_gpu(
    &range_input(10),
    |v| {
      let calls = val(0_u32).make_local_var();
      let r = v.less_than(5).select_branched(
        || {
          add(&calls, val(1));
          (v % val(2))
            .equals(0)
            .select_branched(|| v * val(2), || v + val(100))
        },
        || {
          add(&calls, val(10));
          v.equals(7).select_branched(|| val(7777), || v * v)
        },
      );
      r + calls.load() * val(100000)
    },
    |v| {
      let (r, calls) = if v < 5 {
        (if v % 2 == 0 { v * 2 } else { v + 100 }, 1)
      } else {
        (if v == 7 { 7777 } else { v * v }, 10)
      };
      r + calls * 100000
    },
  )
  .await
}

/// many u32 cases in any order, the empty case does not fall through, and the nested control
/// flow in the cases
#[pollster::test]
async fn switch_u32_gpu() {
  let mut input = range_input(11);
  input.extend([99, 100, 101, u32::MAX - 1, u32::MAX]);
  check_gpu(
    &input,
    |v| {
      let r = val(1_u32).make_local_var();
      switch_by(v)
        .case(100, || r.store(val(1000)))
        .case(0, || r.store(val(10)))
        .case(1, || r.store(val(11)))
        .case(2, || {})
        .case(3, || r.store(v * val(3)))
        .case(5, || r.store(val(15)))
        .case(8, || {
          if_by(v.equals(8), || r.store(val(18))).else_by(|| r.store(val(0)));
        })
        .case(9, || {
          r.store(val(19));
          loop_by(|cx| {
            add(&r, val(1));
            if_by(r.load().greater_equal_than(25), || cx.do_break());
          });
          add(&r, val(100));
        })
        .case(u32::MAX, || r.store(val(2)))
        .end_with_default(|| r.store(val(99)));
      r.load()
    },
    |v| match v {
      100 => 1000,
      0 => 10,
      1 => 11,
      2 => 1,
      3 => 9,
      5 => 15,
      8 => 18,
      9 => 125,
      u32::MAX => 2,
      _ => 99,
    },
  )
  .await
}

/// the i32 selector with the negative and extreme case values
#[pollster::test]
async fn switch_i32_gpu() {
  let mut input: Vec<_> = (-6..6).collect();
  input.extend([i32::MIN, i32::MIN + 1, i32::MAX]);
  check_gpu(
    &input,
    |v| {
      let r = val(0_i32).make_local_var();
      switch_by(v)
        .case(-1, || r.store(val(-10)))
        .case(-3, || r.store(v * val(2)))
        .case(0, || r.store(val(100)))
        .case(2, || r.store(-v))
        .case(i32::MIN, || r.store(val(1)))
        .case(i32::MAX, || r.store(val(2)))
        .case(-4, || {})
        .end_with_default(|| r.store(v / val(2) + val(1000)));
      r.load()
    },
    |v| match v {
      -1 => -10,
      -3 => -6,
      0 => 100,
      2 => -2,
      i32::MIN => 1,
      i32::MAX => 2,
      -4 => 0,
      v => v / 2 + 1000,
    },
  )
  .await
}

/// the switch with only the default case, and the continue inside it
#[pollster::test]
async fn switch_default_only_gpu() {
  check_gpu(
    &range_input(12),
    |v| {
      let i = val(0_u32).make_local_var();
      let sum = val(0_u32).make_local_var();
      switch_by(v).end_with_default(|| sum.store(v * val(10000)));
      loop_by(|cx| {
        let iv = i.load();
        if_by(iv.greater_equal_than(v), || cx.do_break());
        i.store(iv + val(1));
        switch_by(iv).end_with_default(|| {
          if_by((iv % val(2)).equals(0), || cx.do_continue());
          add(&sum, iv);
        });
        add(&sum, val(100));
      });
      sum.load()
    },
    |v| v * 10000 + (0..v).filter(|i| i % 2 == 1).map(|i| i + 100).sum::<u32>(),
  )
  .await
}

/// the switch nested in the cases and the default case of another switch, with the u32 and i32
/// selectors
#[pollster::test]
async fn switch_nested_gpu() {
  check_gpu(
    &range_input(16),
    |v| {
      let r = val(0_u32).make_local_var();
      switch_by(v % val(3))
        .case(0, || {
          switch_by(v / val(3))
            .case(0, || r.store(val(1)))
            .case(1, || r.store(val(2)))
            .end_with_default(|| r.store(val(3)));
        })
        .case(1, || r.store(val(10)))
        .end_with_default(|| {
          switch_by((v / val(3)).into_i32() - val(2))
            .case(-2, || r.store(val(20)))
            .case(0, || r.store(val(21)))
            .end_with_default(|| {
              switch_by(v)
                .case(11, || r.store(val(22)))
                .end_with_default(|| r.store(val(23)));
            });
        });
      r.load()
    },
    |v| match v % 3 {
      0 => match v / 3 {
        0 => 1,
        1 => 2,
        _ => 3,
      },
      1 => 10,
      _ => match (v / 3) as i32 - 2 {
        -2 => 20,
        0 => 21,
        _ if v == 11 => 22,
        _ => 23,
      },
    },
  )
  .await
}

/// the continue inside the switch case and the nested switch case continues the loop
#[pollster::test]
async fn switch_in_loop_continue_gpu() {
  check_gpu(
    &range_input(14),
    |v| {
      let i = val(0_u32).make_local_var();
      let sum = val(0_u32).make_local_var();
      loop_by(|cx| {
        let iv = i.load();
        if_by(iv.greater_equal_than(v), || cx.do_break());
        i.store(iv + val(1));
        switch_by(iv % val(4))
          .case(0, || cx.do_continue())
          .case(1, || add(&sum, val(10)))
          .case(2, || {
            if_by(iv.greater_than(4), || cx.do_continue());
            add(&sum, val(100));
          })
          .end_with_default(|| {
            switch_by(iv)
              .case(3, || cx.do_continue())
              .end_with_default(|| add(&sum, val(1000)));
          });
        add(&sum, val(1));
      });
      sum.load()
    },
    |v| {
      let mut sum = 0;
      for i in 0..v {
        match i % 4 {
          0 => continue,
          1 => sum += 10,
          2 => {
            if i > 4 {
              continue;
            }
            sum += 100;
          }
          _ => {
            if i == 3 {
              continue;
            }
            sum += 1000;
          }
        }
        sum += 1;
      }
      sum
    },
  )
  .await
}

/// the nested loops exit from the nested if chain and by the flag set in a switch case, the
/// variables created in the loop body are initialized in every iteration, and the loop with the
/// break at the end runs once
#[pollster::test]
async fn nested_loop_gpu() {
  check_gpu(
    &range_input(16),
    |v| {
      let total = val(0_u32).make_local_var();
      let i = val(0_u32).make_local_var();
      loop_by(|outer| {
        let iv = i.load();
        if_by(iv.greater_equal_than(v), || outer.do_break());
        i.store(iv + val(1));
        if_by((iv % val(3)).equals(2), || outer.do_continue());
        let j = val(0_u32).make_local_var();
        let stop = val(false).make_local_var();
        loop_by(|inner| {
          let jv = j.load();
          j.store(jv + val(1));
          if_by(jv.greater_equal_than(iv), || inner.do_break())
            .else_if((jv % val(2)).equals(1), || inner.do_continue())
            .else_by(|| {
              if_by(jv.greater_than(8), || inner.do_break()).else_by(|| add(&total, jv * val(10)));
            });
          // the break inside the switch case only exits the switch, so exit by the flag
          switch_by(iv)
            .case(10, || stop.store(jv.equals(6)))
            .end_with_default(|| {});
          if_by(stop.load(), || inner.do_break());
          add(&total, val(1));
        });
        loop_by(|once| {
          add(&total, val(1000));
          once.do_break();
        });
      });
      total.load()
    },
    |v| {
      let mut total = 0;
      for iv in 0..v {
        if iv % 3 == 2 {
          continue;
        }
        let mut j = 0;
        loop {
          let jv = j;
          j += 1;
          if jv >= iv {
            break;
          }
          if jv % 2 == 1 {
            continue;
          }
          if jv > 8 {
            break;
          }
          total += jv * 10;
          if iv == 10 && jv == 6 {
            break;
          }
          total += 1;
        }
        total += 1000;
      }
      total
    },
  )
  .await
}

/// the loop count depends on the input (collatz steps), and is also capped by a counter
#[pollster::test]
async fn loop_data_dependent_exit_gpu() {
  const CAP: u32 = 100;
  check_gpu(
    &range_input(30),
    |v| {
      let n = v.make_local_var();
      let steps = val(0_u32).make_local_var();
      loop_by(|cx| {
        let nv = n.load();
        let finished = nv.less_equal_than(1);
        if_by(finished.or(steps.load().greater_equal_than(CAP)), || {
          cx.do_break()
        });
        if_by((nv % val(2)).equals(0), || n.store(nv / val(2)))
          .else_by(|| n.store(nv * val(3) + val(1)));
        add(&steps, val(1));
      });
      steps.load()
    },
    |v| {
      let (mut n, mut steps) = (v, 0);
      while n > 1 && steps < CAP {
        n = if n % 2 == 0 { n / 2 } else { n * 3 + 1 };
        steps += 1;
      }
      steps
    },
  )
  .await
}

/// the early return from the compute entry in the if, loop and switch case skips the rest of the
/// invocation (the output stays zeroed), and does not affect the other invocations
#[pollster::test]
async fn early_return_gpu() {
  check_gpu(
    &range_input(24),
    |v| {
      if_by(v.equals(3), do_return);
      let r = (v + val(1)).make_local_var();
      loop_by(|cx| {
        let rv = r.load();
        if_by(rv.greater_than(20), || cx.do_break());
        switch_by(v % val(5))
          .case(1, do_return)
          .end_with_default(|| {});
        if_by(rv.equals(9), do_return).else_by(|| r.store(rv * val(2)));
      });
      r.load()
    },
    |v| {
      if v == 3 {
        return 0;
      }
      let mut r = v + 1;
      while r <= 20 {
        if v % 5 == 1 || r == 9 {
          return 0;
        }
        r *= 2;
      }
      r
    },
  )
  .await
}

/// the passed assertions and the never reached unreachable do not change the result
#[pollster::test]
async fn shader_assert_gpu() {
  check_gpu(
    &range_input(16),
    |v| {
      shader_assert(v.less_than(1000));
      let r = val(0_u32).make_local_var();
      switch_by(v % val(2))
        .case(0, || r.store(v / val(2)))
        .case(1, || r.store(v * val(3)))
        .end_with_default(shader_unreachable);
      shader_assert(r.load().less_than(100));
      r.load()
    },
    |v| if v % 2 == 0 { v / 2 } else { v * 3 },
  )
  .await
}

/// the else_if condition is evaluated after the if branch, so it observes the store in the if
/// branch, and it is not evaluated when the if branch is taken
#[pollster::test]
#[ignore = "bug: else_if condition statements are hoisted before the if (naga backend improvement item 6)"]
async fn else_if_condition_after_if_gpu() {
  check_gpu(
    &range_input(3),
    |v| {
      let x = val(0_u32).make_local_var();
      let r = val(0_u32).make_local_var();
      let chain = if_by(v.equals(1), || x.store(val(1)));
      let c = x.load();
      chain.else_if(c.equals(0), || r.store(val(5))).else_over();
      c + r.load() * val(10)
    },
    |v| {
      let x = if v == 1 { 1 } else { 0 };
      let r = if v != 1 && x == 0 { 5 } else { 0 };
      x + r * 10
    },
  )
  .await
}

/// the else_if condition containing a branch (select_branched) is still the else of the chain
#[pollster::test]
#[ignore = "bug: push_else_scope reopens the last If of the block, which is the If built by the else_if condition"]
async fn else_if_condition_with_branch_gpu() {
  check_gpu(
    &range_input(4),
    |v| {
      let r = val(0_u32).make_local_var();
      if_by(v.equals(0), || r.store(val(1)))
        .else_if(
          v.equals(1).select_branched(|| val(true), || v.equals(2)),
          || r.store(val(2)),
        )
        .else_by(|| r.store(val(3)));
      r.load()
    },
    |v| match v {
      0 => 1,
      1 | 2 => 2,
      _ => 3,
    },
  )
  .await
}
