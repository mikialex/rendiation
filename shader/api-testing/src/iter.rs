use rendiation_shader_api::*;

use crate::harness::*;

const ARRAY: [u32; 4] = [10, 20, 30, 40];

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
    keep(ForRange::ranged(vec2_node((val(2), u))).sum());

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
      ShaderStaticArrayReadonlyIter::from_array_clamp_length(readonly_array(), u)
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
        .flat_map(|i| ForRangeState::from_range(vec2_node((i, i + val(2)))))
        .sum(),
    );

    let boxed: Box<dyn ShaderIterator<Item = Node<u32>>> = Box::new(u.into_shader_iter());
    keep(boxed.filter(|i| i.greater_than(val(1))).sum());
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
      .flat_map(|i| ForRangeState::from_range(vec2_node((val(0), i))))
      .for_each(|i, cx| {
        if_by(i.equals(val(3)), || cx.do_continue());
        if_by(i.equals(val(5)), || cx.do_break());
        sum.store(sum.load() + i);
      });
    keep(sum.load());
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
    |v| {
      ForRange::ranged(vec2_node((v, val(4))))
        .map(|_| val(1))
        .sum()
    },
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
          ForRangeState::from_range(vec2_node((i, end)))
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
async fn iter_gpu_array_from_clamp_length() {
  check_iter(
    |v| {
      let array = <[u32; 4]>::create_readonly_view_from_raw_ptr(local_array().raw().clone());
      ShaderStaticArrayReadonlyIter::from_array_clamp_length(array, v)
        .map(|(_, item)| item.load())
        .sum()
    },
    |v| ARRAY.iter().take(v as usize).sum(),
  )
  .await
}
