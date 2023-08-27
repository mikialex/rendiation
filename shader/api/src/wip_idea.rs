use rendiation_webgpu::StorageBufferDataView;

pub trait Monoid {
  fn identity() -> Self;
  fn ops(a: Self, b: Self) -> Self;
}

pub trait GPUParallelComputation {
  type InvocationItem: ShaderNodeType;

  // in these default impls, theoretically we could check the gpu type and detail to compute proper
  // workgroup size or even use different algorithm,
  // for example, check cache size and pre invocation usage? use wrap instruction if available?
  // split to different dispatch if necessary?
  fn collect_storage_buffer(&self) -> StorageBufferDataView<[Self::InvocationItem]> {
    // default impl
  }

  fn map<R>(
    self,
    f: impl Fn(Self::InvocationItem) -> R,
  ) -> impl GPUParallelComputation<InvocationItem = R> {
    //
  }
  fn prefix_scan(self, inclusive: bool) -> impl GPUParallelComputation
  where
    Node<Self::InvocationItem>: Monoid,
  {
    //
  }

  fn reduce_to_storage_buffer(&self) -> StorageBufferDataView<Self::InvocationItem>
  where
    Node<Self::InvocationItem>: Monoid,
  {
    // default impl
  }
}

pub trait GPUParallelSplit {
  type InvocationItem: ShaderNodeType;
  type Child;
  type Context;
  fn process_and_split(
    item: Self::InvocationItem,
    cx: &Self::Context,
    child_collector: impl Fn(Self::Child),
  );
}

pub trait GPURecursiveFunction {
  type Input;
  type Output;
  type Child;
  fn process_and_split(input: Self::Input, child_collector: impl Fn(Self::Child)) -> Self::Output;
}

// pub struct HierarchyCullingNode {}

struct ShaderMap<T, F> {
  inner: T,
  shader_map: F,
}
impl<T, F> ShaderIterator for ShaderMap<T, F> {
  //
}

impl<T> GPUParallelComputation for StorageBufferDataView<T> {
  //
}

struct GPUParallelMap<T, F> {
  inner: T,
  mapper: F,
}
impl<T, F> GPUParallelComputation for GPUParallelMap<T, F> {
  //
}

struct GPUParallelPrefixScan<T> {
  inner: T,
}
impl<T> GPUParallelComputation for GPUParallelPrefixScan<T>
where
  Node<Self::InvocationItem>: Monoid,
{
  //
}
