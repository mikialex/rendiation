use std::any::TypeId;

use crate::*;

pub trait InvocationAccessBehavior<T>: Clone {
  fn access_behavior(
    &self,
    source: &dyn DeviceInvocation<Node<T>>,
    current: Node<Vec3<u32>>,
  ) -> (Node<T>, Node<bool>);
  fn resize_scope(&self, size: Node<Vec3<u32>>) -> Node<Vec3<u32>> {
    size
  }
  fn resize_work_size(&self, size: u32) -> u32 {
    size
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub enum OutBoundsBehavior<T> {
  ClampBorder,
  Const(Arc<dyn Fn() -> Node<T>>, TypeId),
}

impl<T> OutBoundsBehavior<T> {
  pub fn from_const<F: Fn() -> Node<T> + 'static>(f: F) -> Self {
    Self::Const(Arc::new(f), TypeId::of::<F>())
  }
}

impl<T> Hash for OutBoundsBehavior<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    core::mem::discriminant(self).hash(state);
    if let OutBoundsBehavior::Const(_, id) = self {
      id.hash(state);
    }
  }
}

impl<T> OutBoundsBehavior<T> {
  pub fn sample(&self, border: Node<T>) -> Node<T> {
    match self {
      OutBoundsBehavior::ClampBorder => border,
      OutBoundsBehavior::Const(v, _) => v(),
    }
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
#[derivative(Hash(bound = ""))]
pub struct DeviceInvocationOffset<T> {
  pub offset: i32,
  pub ob: OutBoundsBehavior<T>,
  pub size_expand: u32,
}

impl<T> InvocationAccessBehavior<T> for DeviceInvocationOffset<T>
where
  T: ShaderSizedValueNodeType,
{
  fn access_behavior(
    &self,
    source: &dyn DeviceInvocation<Node<T>>,
    current: Node<Vec3<u32>>,
  ) -> (Node<T>, Node<bool>) {
    let size = source.invocation_size().x().into_i32();

    let current = current.x(); // todo three dimension
    let current = current.into_i32(); // todo overflow check
    let target = current + val(self.offset);

    let output = zeroed_val::<T>().make_local_var();
    if_by(target.less_than(val(0)), || {
      output.store(self.ob.sample(source.start_point()));
    })
    .else_if(target.greater_equal_than(size), || {
      output.store(self.ob.sample(source.end_point()));
    })
    .else_by(|| {
      let _target = target.into_u32(); // todo overflow check
      let _target: Node<Vec3<u32>> = (_target, val(0), val(0)).into();
      output.store(source.invocation_logic(_target).0)
    });

    // todo, should return inner valid
    (output.load(), val(true))
  }

  fn resize_scope(&self, size: Node<Vec3<u32>>) -> Node<Vec3<u32>> {
    size + val(Vec3::new(self.size_expand, 0, 0))
  }
  fn resize_work_size(&self, size: u32) -> u32 {
    size + self.size_expand
  }
}

struct DeviceInvocationAccessBehaviorImpl<T, F>(Box<dyn DeviceInvocation<Node<T>>>, F);

impl<T, F> DeviceInvocation<Node<T>> for DeviceInvocationAccessBehaviorImpl<T, F>
where
  T: ShaderSizedValueNodeType,
  F: InvocationAccessBehavior<T>,
{
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    self.1.access_behavior(&self.0, logic_global_id)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.1.resize_scope(self.0.invocation_size())
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "F: Clone"))]
pub struct DeviceParallelComputeCustomInvocationBehavior<T, F> {
  pub source: Box<dyn DeviceInvocationComponent<Node<T>>>,
  pub behavior: F,
}

impl<T: 'static, F: Hash + 'static> ShaderHashProvider
  for DeviceParallelComputeCustomInvocationBehavior<T, F>
{
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.behavior.hash(hasher);
    self.source.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T, F> DeviceInvocationComponent<Node<T>>
  for DeviceParallelComputeCustomInvocationBehavior<T, F>
where
  T: ShaderSizedValueNodeType,
  F: Hash + Clone + InvocationAccessBehavior<T> + 'static,
{
  fn clone_boxed(&self) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(self.clone())
  }
  fn result_size(&self) -> u32 {
    self.behavior.resize_work_size(self.source.result_size())
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    Box::new(DeviceInvocationAccessBehaviorImpl(
      self.source.build_shader(builder),
      self.behavior.clone(),
    ))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.source.bind_input(builder);
  }
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.source.requested_workgroup_size()
  }

  fn work_size(&self) -> Option<u32> {
    self
      .behavior
      .resize_work_size(self.source.work_size()?)
      .into()
  }
}

impl<T, F> DeviceInvocationComponentIO<T> for DeviceParallelComputeCustomInvocationBehavior<T, F>
where
  T: ShaderSizedValueNodeType,
  F: Hash + Clone + InvocationAccessBehavior<T> + 'static,
{
}

#[pollster::test]
async fn test1() {
  gpu_test_scope(async |cx| {
    let input = [0, 1, 2, 3, 4, 5].to_vec();
    let expect = [3, 4, 5, 5, 5, 5].to_vec();

    let input = slice_into_compute(&input, cx);

    input
      .offset_access(3, OutBoundsBehavior::ClampBorder, 0)
      .run_test(cx, &expect)
      .await
  })
  .await
}

#[allow(unused_macros)]
macro_rules! gpu_cx {
  ($name: tt) => {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    let mut encoder = gpu.create_encoder();
    let mut $name = DeviceParallelComputeCtx::new(&gpu, &mut encoder);
  };
}

#[pollster::test]
async fn test2() {
  // let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  // let mut encoder = gpu.create_encoder();
  // let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder);
  gpu_cx!(cx);

  let input = [0, 1, 2, 3, 4, 5].to_vec();
  let expect = [0, 0, 0, 1, 2, 3, 4].to_vec();

  let input = slice_into_compute(&input, &mut cx);

  input
    .offset_access(-2, OutBoundsBehavior::ClampBorder, 1)
    .run_test(&mut cx, &expect)
    .await;
}

#[pollster::test]
async fn test3() {
  gpu_test_scope(async |cx| {
    let input = [0, 1, 2, 3, 4, 5].to_vec();
    let expect = [6, 6, 0, 1, 2, 3].to_vec();

    let input = slice_into_compute(&input, cx);

    input
      .offset_access(-2, OutBoundsBehavior::from_const(|| val(6)), 0)
      .run_test(cx, &expect)
      .await
  })
  .await
}
