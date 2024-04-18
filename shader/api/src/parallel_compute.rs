use crate::*;

pub trait DeviceLinearIdentity {
  fn into_id(node: Node<Self>) -> Node<u32>;
  fn from_id(id: Node<u32>) -> Node<Self>;
}

/// pure shader structures
pub trait DeviceCollection<K, V> {
  /// should not contain any side effects
  fn device_access(&self, key: Node<K>) -> (Node<V>, Node<bool>);
}

/// degenerated DeviceCollection, K is the global invocation id in compute ctx
pub trait DeviceInvocationAccess<V> {
  fn device_invocation_access(&self) -> (Node<V>, Node<bool>);
}

// struct StorageBufferArray<V>(V);

// impl<V> StorageBufferArray<V> {
//   pub fn register(cx: &mut ShaderComputePipelineBuilder) -> Box<dyn DeviceCollection<u32, V>> {
//     todo!()
//   }
// }

pub trait DeviceParallelCompute<V> {
  fn build_invocation_access(
    &self,
    cx: &mut usize,
  ) -> Box<dyn FnOnce() -> Box<dyn DeviceInvocationAccess<V>>>;
}

impl<T: ShaderNodeType> DeviceCollection<u32, T> for Node<ShaderReadOnlyStoragePtr<[T]>> {
  fn device_access(&self, key: Node<u32>) -> (Node<T>, Node<bool>) {
    // if_by(condition, logic)
    (self.index(key).load(), val(true))
  }
}

pub trait DeviceMonoid {
  fn identity() -> Node<Self>;
  fn combine(a: Node<Self>, b: Node<Self>) -> Node<Self>;
}

// pub trait IntoLeftValue {
//   type LeftValue;
//   fn make_local_var(self) -> Self::LeftValue;
// }

// pub trait IntoRightValue {
//   type RightValue;
//   fn load_local_var(self) -> Self::RightValue;
// }

// pub trait IntoShaderValue {
//   type ShaderValue: ShaderNodeType;
//   fn into_shader_value(self) -> Node<Self::ShaderValue>;
//   fn from_shader_value(sv: Node<Self::ShaderValue>) -> Self;
// }

// struct WorkGroupPrefixSum<V> {
//   upstream: Box<dyn DeviceParallelCompute<Node<V>>>,
//   workgroup_usage: Node<ShaderWorkGroupPtr<[V; 128]>>,
// }

// impl<K, V> GPUParallelComputation<K, Node<V>> for WorkGroupPrefixSum<K, V>
// where
//   V: ShaderNodeType + DeviceMonoid,
// {
//   fn thread_logic(&self , key: K) -> Node<V> {
//     let input = self.upstream.thread_logic(key);
//     let shared = self.workgroup_usage;

//     let local_id = local_invocation_id().x();

//     let value = input.make_local_var();

//     shared.index(local_id).store(value.load());

//     128.ilog2().into_shader_iter().for_each(|i, _| {
//       workgroup_barrier();

//       if_by(local_id.greater_equal_than(val(1) << i), || {
//         let a = value.load();
//         let b = shared.index(local_id - (val(1) << i)).load();
//         let combined = V::combine(a, b);
//         value.store(combined)
//       });

//       workgroup_barrier();
//       shared.index(local_id).store(value.load())
//     });

//     value.load()
//   }
// }

// pub trait PCWorkLoadSize {
//   fn size(&self) -> u32;
// }

pub trait DeviceParallelComputeExt<V> {
  // fn write_into_storage_buffer(self) -> StorageBufferDataView<[V]>
  // fn host_read(self) -> Box<dyn Future<Output = Box<dyn VirtualCollection<u32, V>>>>;

  // fn map<R>(self, f: impl Fn(V) -> R) -> impl GPUParallelComputation<K, R> {
  //   //
  // }

  // fn workgroup_prefix_scan(
  //   self,
  //   inclusive: bool,
  //   workgroup_size: u32,
  //   cx: &ShaderComputePipelineBuilder,
  // ) -> impl DeviceParallelCompute<V>
  // where
  //   V: DeviceMonoid,
  // {
  //   //
  // }

  // fn prefix_scan(self, inclusive: bool) -> impl GPUParallelComputation<K, V>
  // where
  //   V: Monoid,
  // {
  //   //
  // }

  // fn reduce_to_storage_buffer(&self) -> StorageBufferDataView<V>
  // where
  //   V: Monoid,
  // {
  //   // default impl
  // }
}

// pub trait GPUParallelSplit {
//   type InvocationItem: ShaderNodeType;
//   type Child;
//   type Context;
//   fn process_and_split(
//     item: Self::InvocationItem,
//     cx: &Self::Context,
//     child_collector: impl Fn(Self::Child),
//   );
// }

// enum Child {
//   WorkA(Node<f32>),
//   WorkB(Node<f32>),
// }

// pub trait GPURecursiveProcess {
//   type Input;
//   type Child;
//   type Context;
//   fn process_and_split(
//     input: Self::Input,
//     cx: &Self::Context,
//     child_collector: impl Fn(Self::Child),
//   );
// }

// pub struct HierarchyCullingNode {}

// impl<T> GPUParallelComputation for StorageBufferDataView<T> {
//   //
// }

// struct GPUParallelMap<T, F> {
//   inner: T,
//   mapper: F,
// }
// impl<T, F> GPUParallelComputation for GPUParallelMap<T, F> {
//   //
// }

// struct GPUParallelPrefixScan<T> {
//   inner: T,
// }
// impl<T> GPUParallelComputation for GPUParallelPrefixScan<T>
// where
//   Node<Self::InvocationItem>: Monoid,
// {
//   //
// }
