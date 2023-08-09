use rendiation_webgpu::StorageBufferDataView;

pub trait ShaderIterator: ShaderGraphNodeType {
  type Item: ShaderGraphNodeType;
  // we do not have sum type(enum) in shader!;
  fn shader_has_next(self: Node<Self>) -> Node<bool>;
  fn shader_next(mut_self: MutableNode<Self>) -> Node<Self::Item>;

  fn map(self) -> impl ShaderIterator {
    //
  }

  fn for_each(self, visitor: impl Fn(Self::Item)) {
    let has_next_f = define_fn(|cx| {
      let iter_self = cx.define_parameter();
      Self::shader_has_next(iter_self)
    });

    let next_f = define_fn(|cx| {
      let iter_self_mut = cx.define_parameter();
      cx.do_return(Self::shader_next(iter_self));
    });

    let shader_iter = self.mutable();
    while_by(|cx| {
      let has_next = has_next_f();
      if_by(has_next.not(), || cx.do_break());
      visitor(next_f(shader_iter))
    })
  }
}

pub trait Monoid {
  fn identity() -> Self;
  fn ops(a: Self, b: Self) -> Self;
}

pub trait ParallelComputation {
  type InvocationItem: ShaderGraphNodeType;
  fn invocation_count(&self) -> usize;

  // in these default impls, theoretically we could check the gpu type and detail to compute proper
  // workgroup size or even use different algorithm,
  // for example, check cache size and pre invocation usage? use wrap instruction if available?
  // split to different dispatch if necessary?
  fn collect_storage_buffer(&self) -> StorageBufferDataView<[Self::InvocationItem]> {
    // default impl
  }

  fn map(self) -> impl ParallelComputation {
    //
  }
  fn prefix_scan(self) -> impl ParallelComputation
  where
    Node<Self::InvocationItem>: Monoid,
  {
    //
  }

  fn reduce_to_storage_buffer(&self, inclusive: bool) -> StorageBufferDataView<Self::InvocationItem>
  where
    Node<Self::InvocationItem>: Monoid,
  {
    // default impl
  }
}

struct ShaderMap<T, F> {
  inner: T,
  shader_map: F,
}
impl<T, F> ShaderIterator for ShaderMap<T, F> {
  //
}

impl<T> ParallelComputation for StorageBufferDataView<T> {
  //
}

struct GPUParallelMap<T, F> {
  inner: T,
  mapper: F,
}
impl<T, F> ParallelComputation for GPUParallelMap<T, F> {
  //
}

struct GPUParallelPrefixScan<T> {
  inner: T,
}
impl<T> ParallelComputation for GPUParallelPrefixScan<T>
where
  Node<Self::InvocationItem>: Monoid,
{
  //
}
