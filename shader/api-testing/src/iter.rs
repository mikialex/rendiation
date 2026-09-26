use rendiation_shader_api::*;

use crate::harness::*;

const ARRAY: [u32; 4] = [10, 20, 30, 40];

#[shader_struct]
#[derive(Clone, Copy)]
pub struct IterPair {
  pub sum: u32,
  pub count: u32,
}

fn local_array() -> ShaderPtrOf<[u32; 4]> {
  let array = make_local_var::<[u32; 4]>();
  for (i, v) in ARRAY.iter().enumerate() {
    array.index(i as u32).store(val(*v));
  }
  array
}

/// all the iteration sources
#[test]
fn iter_sources() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;

    keep(4_u32.into_shader_iter().sum());
    keep(u.into_shader_iter().sum());
    keep(vec2_node((u, val(8))).into_shader_iter().sum());
    keep(ShaderRangeIter::new(ShaderRange::new(val(2), u)).sum());
    keep(
      ShaderRange::from_vec2(vec2_node((val(2), u)))
        .into_shader_iter()
        .sum(),
    );
    keep((val(2)..u).into_shader_iter().sum());
    keep((2..8).into_shader_iter().sum());

    keep(
      local_array()
        .into_shader_iter()
        .map(|(_, v)| v.load())
        .sum(),
    );

    let readonly_array =
      || <[u32; 4]>::create_readonly_view_from_raw_ptr(local_array().raw().clone());
    keep(
      readonly_array()
        .into_shader_iter()
        .map(|(_, v)| v.load())
        .sum(),
    );
    keep(
      ShaderIndexIter::with_len_clamp(readonly_array(), u)
        .map(|(_, v)| v.load())
        .sum(),
    );

    let storage = fake_storage_buffer::<[u32]>(0);
    keep(
      storage
        .clone()
        .into_shader_iter()
        .map(|(_, v)| v.load())
        .sum(),
    );
    let readonly_storage = storage.into_readonly_view();
    keep(
      readonly_storage
        .into_shader_iter()
        .map(|(_, v)| v.load())
        .sum(),
    );
  });
}

/// all the adaptors, the closure parameter types are inferred from the iterator item
#[test]
fn iter_adaptors() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;

    keep(u.into_shader_iter().map(|i| i.min(val(3))).sum());
    keep(u.into_shader_iter().filter(|i| i.less_than(val(3))).sum());
    keep(
      u.into_shader_iter()
        .filter_map(|i| (i.less_than(val(3)), i.into_f32()))
        .sum(),
    );
    keep(
      u.into_shader_iter()
        .zip(4_u32.into_shader_iter())
        .map(|(a, b)| a.max(b))
        .sum(),
    );
    keep(
      u.into_shader_iter()
        .enumerate()
        .take_while(|(i, v)| i.less_than(*v))
        .map(|(i, _)| i)
        .sum(),
    );
    keep(
      local_array()
        .into_shader_iter()
        .clamp_by(u)
        .map(|(_, v)| v.load())
        .sum(),
    );
    keep(
      u.into_shader_iter()
        .filter(|i| i.greater_than(val(1)))
        .take(val(3))
        .sum(),
    );
    // the input item of filter_map is not stored, so the pointer item is allowed
    keep(
      local_array()
        .into_shader_iter()
        .filter_map(|(i, v)| (i.less_than(u), v.load()))
        .sum(),
    );
    keep(
      u.into_shader_iter()
        .flat_map(|i| ShaderRange::from_vec2(vec2_node((i, i + val(2)))))
        .sum(),
    );

    let boxed: Box<dyn ShaderIterator<Item = Node<u32>>> = Box::new(u.into_shader_iter());
    keep(boxed.filter(|i| i.greater_than(val(1))).sum());
  });
}

/// the consumers, and the right value items (tuple and the expanded struct) through the adaptors
#[test]
fn iter_consumers_and_right_values() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;

    keep(u.into_shader_iter().count());
    keep(u.into_shader_iter().any(|i| i.equals(val(3))));
    keep(u.into_shader_iter().all(|i| i.less_than(val(3))));
    keep(
      u.into_shader_iter()
        .find(|i| i.greater_than(val(3)))
        .unwrap_or(val(0)),
    );
    keep(
      u.into_shader_iter()
        .position(|i| i.greater_than(val(3)))
        .unwrap_or(val(0)),
    );
    keep(u.into_shader_iter().fold(val(1), |acc, i| acc * i));

    let init = ENode::<IterPair> {
      sum: val(0),
      count: val(0),
    };
    let pair = u.into_shader_iter().fold(init, |acc, i| ENode::<IterPair> {
      sum: acc.sum + i,
      count: acc.count + val(1),
    });
    keep(pair.sum + pair.count);

    // the expanded struct and the tuple of 3 are right values, so they can be filtered
    let pairs = u
      .into_shader_iter()
      .map(|i| ENode::<IterPair> { sum: i, count: i });
    keep(
      pairs
        .filter(|p| p.sum.greater_than(val(1)))
        .map(|p| p.count)
        .sum(),
    );
    keep(
      u.into_shader_iter()
        .map(|i| (i, i + val(1), i + val(2)))
        .take_while(|(a, _, _)| a.less_than(val(4)))
        .map(|(a, b, c)| a + b + c)
        .sum(),
    );

    // zip takes any iterable
    keep(u.into_shader_iter().zip(4_u32).map(|(a, b)| a + b).sum());
  });
}

/// the break and continue in the visitor target the for_each loop, the internal search loop of
/// the adaptors does not affect them
#[test]
fn iter_visitor_loop_control() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let sum = val(0_u32).make_local_var();
    u.into_shader_iter()
      .filter(|i| i.greater_than(val(1)))
      .flat_map(|i| ShaderRange::from_vec2(vec2_node((val(0), i))))
      .for_each(|i, cx| {
        if_by(i.equals(val(3)), || cx.do_continue());
        if_by(i.equals(val(5)), || cx.do_break());
        sum.store(sum.load() + i);
      });
    keep(sum.load());
  });
}

/// the iterator created outside of the loop and iterated inside the loop is rejected, its state is
/// not reset for each loop execution
#[test]
#[should_panic(expected = "created outside of the loop and iterated inside the loop")]
fn iter_created_outside_loop() {
  build_compute(|builder| {
    let u = runtime_values(builder).u;
    let iter = u.into_shader_iter();
    loop_by(|cx| {
      iter.for_each(|_, _| {});
      cx.do_break();
    });
  });
}

/// the same case when the outer loop is another iteration
#[test]
#[should_panic(expected = "created outside of the loop and iterated inside the loop")]
fn iter_created_outside_outer_iteration() {
  build_compute(|builder| {
    let u = runtime_values(builder).u;
    let inner = local_array().into_shader_iter();
    u.into_shader_iter().for_each(|_, _| {
      keep(inner.count());
    });
  });
}

/// the iteration inside the switch case or the branch of the iterator created outside is fine, and
/// the iterator created inside the loop is fine
#[test]
fn iter_scope_allowed() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;

    let iter = u.into_shader_iter();
    switch_by(u)
      .case(0, || keep(iter.sum()))
      .end_with_default(|| {});

    let iter = u.into_shader_iter();
    if_by(u.equals(val(1)), || keep(iter.count()));

    u.into_shader_iter().for_each(|i, _| {
      let inner = (val(0)..i).into_shader_iter().enumerate();
      keep(inner.map(|(a, b)| a + b).sum());
    });
  });
}

/// The inputs of the GPU execution tests, each value is the parameter of one invocation.
const INPUT: &[u32] = &[0, 1, 2, 3, 4, 5, 7, 10];

/// Run the shader logic on the GPU for each input value, and compare with the cpu reference logic.
async fn check_iter(shader: impl Fn(Node<u32>) -> Node<u32> + 'static, cpu: impl Fn(u32) -> u32) {
  let expect: Vec<_> = INPUT.iter().map(|v| cpu(*v)).collect();
  assert_eq!(gpu_map(INPUT, shader).await, expect);
}

#[pollster::test]
async fn iter_gpu_count() {
  check_iter(|v| v.into_shader_iter().sum(), |v| (0..v).sum()).await
}

/// the range that start is larger than the end is empty, instead of looping forever
#[pollster::test]
async fn iter_gpu_range() {
  check_iter(
    |v| (v..val(4)).into_shader_iter().map(|_| val(1)).sum(),
    |v| (v..4).count() as u32,
  )
  .await
}

/// all the matched items are visited, not only the last one
#[pollster::test]
async fn iter_gpu_filter() {
  check_iter(
    |v| {
      v.into_shader_iter()
        .filter(|i| (*i % val(3)).not_equals(val(0)))
        .map(|i| i * val(2))
        .sum()
    },
    |v| (0..v).filter(|i| i % 3 != 0).map(|i| i * 2).sum(),
  )
  .await
}

#[pollster::test]
async fn iter_gpu_filter_map() {
  check_iter(
    |v| {
      v.into_shader_iter()
        .filter_map(|i| ((i % val(3)).equals(val(0)), i + val(1)))
        .sum()
    },
    |v| (0..v).filter(|i| i % 3 == 0).map(|i| i + 1).sum(),
  )
  .await
}

/// the empty inner iterator is skipped instead of ending the iteration
#[pollster::test]
async fn iter_gpu_flat_map() {
  check_iter(
    |v| {
      v.into_shader_iter()
        .flat_map(|i| {
          let end = i + (i % val(3)) * val(2);
          ShaderRange::from_vec2(vec2_node((i, end)))
        })
        .sum()
    },
    |v| (0..v).flat_map(|i| i..i + (i % 3) * 2).sum(),
  )
  .await
}

#[pollster::test]
async fn iter_gpu_zip_enumerate_take_while() {
  check_iter(
    |v| {
      v.into_shader_iter()
        .map(|i| i * val(3))
        .zip(6_u32.into_shader_iter())
        .enumerate()
        .take_while(|(idx, _)| idx.less_than(val(5)))
        .map(|(idx, (a, b))| idx + a + b)
        .sum()
    },
    |v| {
      (0..v)
        .map(|i| i * 3)
        .zip(0..6)
        .enumerate()
        .take_while(|(idx, _)| *idx < 5)
        .map(|(idx, (a, b))| idx as u32 + a + b)
        .sum()
    },
  )
  .await
}

#[pollster::test]
async fn iter_gpu_array_clamp() {
  check_iter(
    |v| {
      local_array()
        .into_shader_iter()
        .clamp_by(v)
        .map(|(_, item)| item.load())
        .sum()
    },
    |v| ARRAY.iter().take(v as usize).sum(),
  )
  .await
}

/// the length clamp larger than the array length does not access the array out of bounds
#[pollster::test]
async fn iter_gpu_array_with_len_clamp() {
  check_iter(
    |v| {
      let array = <[u32; 4]>::create_readonly_view_from_raw_ptr(local_array().raw().clone());
      ShaderIndexIter::with_len_clamp(array, v)
        .map(|(_, item)| item.load())
        .sum()
    },
    |v| ARRAY.iter().take(v as usize).sum(),
  )
  .await
}

/// the map closure only runs for the valid items, not for the poll that ends the iteration
#[pollster::test]
async fn iter_gpu_map_only_valid_item() {
  check_iter(
    |v| {
      let calls = val(0_u32).make_local_var();
      v.into_shader_iter()
        .map(|i| {
          calls.store(calls.load() + val(1));
          i
        })
        .for_each(|_, _| {});
      calls.load()
    },
    |v| v,
  )
  .await
}

/// the array item index is never out of range, even when the take count is larger than the array
#[pollster::test]
async fn iter_gpu_array_item_in_range() {
  check_iter(
    |v| {
      let out_of_range = val(0_u32).make_local_var();
      let sum = local_array()
        .into_shader_iter()
        .take(v)
        .map(|(i, item)| {
          if_by(i.greater_equal_than(val(ARRAY.len() as u32)), || {
            out_of_range.store(out_of_range.load() + val(1));
          });
          item.load()
        })
        .sum();
      sum + out_of_range.load() * val(1000)
    },
    |v| ARRAY.iter().take(v as usize).sum(),
  )
  .await
}

#[pollster::test]
async fn iter_gpu_take() {
  check_iter(
    |v| {
      v.into_shader_iter()
        .filter(|i| (*i % val(2)).equals(val(1)))
        .take(val(3))
        .sum()
    },
    |v| (0..v).filter(|i| i % 2 == 1).take(3).sum(),
  )
  .await
}

#[pollster::test]
async fn iter_gpu_filter_map_pointer_item() {
  check_iter(
    |v| {
      local_array()
        .into_shader_iter()
        .filter_map(|(i, item)| ((i * val(2)).less_than(v), item.load()))
        .sum()
    },
    |v| {
      (0..ARRAY.len() as u32)
        .filter(|i| i * 2 < v)
        .map(|i| ARRAY[i as usize])
        .sum()
    },
  )
  .await
}

#[pollster::test]
async fn iter_gpu_range_source() {
  check_iter(
    |v| (v..val(6)).into_shader_iter().sum() + (2..5).into_shader_iter().sum() * val(100),
    |v| (v..6).sum::<u32>() + (2..5).sum::<u32>() * 100,
  )
  .await
}

/// the count does not construct the items, so the map closure does not run
#[pollster::test]
async fn iter_gpu_count_consumer() {
  check_iter(
    |v| {
      let calls = val(0_u32).make_local_var();
      let count = v
        .into_shader_iter()
        .filter(|i| (*i % val(2)).equals(val(1)))
        .map(|i| {
          calls.store(calls.load() + val(1));
          i
        })
        .count();
      count + calls.load() * val(1000)
    },
    |v| (0..v).filter(|i| i % 2 == 1).count() as u32,
  )
  .await
}

#[pollster::test]
async fn iter_gpu_any_all() {
  check_iter(
    |v| {
      let any = v.into_shader_iter().any(|i| i.equals(val(3)));
      let all = v.into_shader_iter().all(|i| i.less_than(val(5)));
      any.into_u32() + all.into_u32() * val(2)
    },
    |v| (0..v).any(|i| i == 3) as u32 + (0..v).all(|i| i < 5) as u32 * 2,
  )
  .await
}

#[pollster::test]
async fn iter_gpu_find_position() {
  check_iter(
    |v| {
      let found = v
        .into_shader_iter()
        .find(|i| (*i * *i).greater_than(val(10)));
      let position = local_array()
        .into_shader_iter()
        .take(v)
        .position(|(_, item)| item.load().greater_than(val(25)));
      found.unwrap_or(val(99)) + position.unwrap_or(val(99)) * val(1000)
    },
    |v| {
      let found = (0..v).find(|i| i * i > 10).unwrap_or(99);
      let position = ARRAY.iter().take(v as usize).position(|item| *item > 25);
      found + position.map(|p| p as u32).unwrap_or(99) * 1000
    },
  )
  .await
}

#[pollster::test]
async fn iter_gpu_fold_struct() {
  check_iter(
    |v| {
      let init = ENode::<IterPair> {
        sum: val(0),
        count: val(0),
      };
      let pair = v
        .into_shader_iter()
        .filter(|i| i.greater_than(val(1)))
        .fold(init, |acc, i| ENode::<IterPair> {
          sum: acc.sum + i,
          count: acc.count + val(1),
        });
      pair.sum + pair.count * val(1000)
    },
    |v| {
      let items = (0..v).filter(|i| *i > 1);
      items.clone().sum::<u32>() + items.count() as u32 * 1000
    },
  )
  .await
}

/// the tuple of 3 and the expanded struct items are carried through the filter like adaptors
#[pollster::test]
async fn iter_gpu_right_value_items() {
  check_iter(
    |v| {
      let tuple_sum = v
        .into_shader_iter()
        .map(|i| (i, i * val(2), i * val(3)))
        .filter(|(a, _, _)| (*a % val(2)).equals(val(1)))
        .map(|(a, b, c)| a + b + c)
        .sum();
      let struct_sum = v
        .into_shader_iter()
        .filter_map(|i| {
          let pair = ENode::<IterPair> {
            sum: i,
            count: i * val(10),
          };
          (i.less_than(val(4)), pair)
        })
        .map(|pair| pair.sum + pair.count)
        .sum();
      tuple_sum + struct_sum * val(1000)
    },
    |v| {
      let tuple_sum: u32 = (0..v).filter(|a| a % 2 == 1).map(|a| a * 6).sum();
      let struct_sum: u32 = (0..v).filter(|i| *i < 4).map(|i| i * 11).sum();
      tuple_sum + struct_sum * 1000
    },
  )
  .await
}
