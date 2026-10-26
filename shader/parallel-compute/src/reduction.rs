use crate::*;

#[derive_where(Clone)]
pub struct WorkGroupReductionCompute<T, S> {
  pub workgroup_size: u32,
  pub reduction_logic: PhantomData<S>,
  pub upstream: Box<dyn ComputeComponent<Node<T>>>,
}

impl<T: 'static, S: 'static> ShaderHashProvider for WorkGroupReductionCompute<T, S> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.workgroup_size);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T, S> ComputeComponent<Node<T>> for WorkGroupReductionCompute<T, S>
where
  T: ShaderScalarOrVec,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn result_size(&self) -> u32 {
    // todo, fix
    self.upstream.result_size()
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    Some(self.workgroup_size)
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    assert!(
      self.workgroup_size.is_power_of_two(),
      "workgroup reduction requires power of two workgroup size"
    );
    let local_id = builder.local_invocation_id().x();
    let shared = builder.define_workgroup_shared_var_host_size_array::<T>(self.workgroup_size);

    let iter = self.workgroup_size.ilog2();

    self
      .upstream
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |upstream, id| {
        let (input, valid) = upstream.invocation_logic(id);

        let input = valid.select(input, S::identity());

        shared.index(local_id).store(input);

        // each step combines adjacent pairs to keep the original order, because the monoid is
        // not required to be commutative.
        let combined = S::identity().make_local_var();
        iter.into_shader_iter().for_each(|i, _| {
          workgroup_barrier();

          let active_count = val(1) << (val(iter) - i - val(1));
          let is_active = local_id.less_than(active_count);

          if_by(is_active, || {
            let a = shared.index(local_id * val(2)).load();
            let b = shared.index(local_id * val(2) + val(1)).load();
            combined.store(S::combine(a, b));
          });

          workgroup_barrier();

          if_by(is_active, || {
            shared.index(local_id).store(combined.load());
          });
        });
        workgroup_barrier();

        let result = local_id
          .equals(0)
          .select_branched(|| shared.index(0).load(), || S::identity());

        (result, local_id.equals(0))
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn work_size(&self) -> Option<u32> {
    self.upstream.work_size()
  }

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<T>>> {
    Box::new(self.clone())
  }
}

impl<T, S> ComputeComponentIO<T> for WorkGroupReductionCompute<T, S>
where
  T: ShaderScalarOrVec,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn use_materialize_storage_buffer_into(
    &self,
    target: StorageBufferDataView<[T]>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    use_custom_write_into_storage_buffer(
      self,
      cx,
      move |global_id| (global_id, val(true)),
      Arc::new(LinearWriterHash),
      target,
    )
  }
}

#[pollster::test]
async fn test1() {
  gpu_cx!(cx);
  let input = vec![1_u32; 8];

  let expect = vec![4, 0, 0, 0, 4, 0, 0, 0];

  let workgroup_size = 4;
  let input = slice_into_compute(&input, cx);

  input
    .workgroup_scope_reduction::<AdditionMonoid<_>>(workgroup_size, cx)
    .run_test(cx, &expect)
    .await
}

#[pollster::test]
async fn test2() {
  gpu_cx!(cx);
  let input = vec![1_u32; 70];

  let expect = vec![70];

  let workgroup_size = 32;
  let input = slice_into_compute(&input, cx);

  input
    .segmented_reduction::<AdditionMonoid<_>>(workgroup_size, workgroup_size, cx)
    .run_test(cx, &expect)
    .await
}

/// affine map composition: (a, b) represents x * a + b, apply left first. It is associative but
/// not commutative.
#[cfg(test)]
#[derive(Default)]
struct TestAffineComposeMonoid;
#[cfg(test)]
impl DeviceMonoidLogic for TestAffineComposeMonoid {
  type Data = Vec2<u32>;
  fn identity() -> Node<Vec2<u32>> {
    val(Vec2::new(1, 0))
  }
  fn combine(a: Node<Vec2<u32>>, b: Node<Vec2<u32>>) -> Node<Vec2<u32>> {
    (a.x() * b.x(), a.y() * b.x() + b.y()).into()
  }
}

#[pollster::test]
async fn test_reduction_non_commutative() {
  gpu_cx!(cx);
  let input: Vec<_> = (0..8).map(|i| Vec2::new(i + 2, i)).collect();

  let combined = input.iter().fold(Vec2::new(1_u32, 0), |a, b| {
    Vec2::new(
      a.x.wrapping_mul(b.x),
      a.y.wrapping_mul(b.x).wrapping_add(b.y),
    )
  });
  let mut expect = vec![Vec2::zero(); 8];
  expect[0] = combined;

  let input = slice_into_compute(&input, cx);
  input
    .workgroup_scope_reduction::<TestAffineComposeMonoid>(8, cx)
    .run_test(cx, &expect)
    .await
}
