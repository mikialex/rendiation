use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod io;
mod map;

/// pure shader structures
pub trait DeviceCollection<K, V> {
  /// should not contain any side effects
  fn device_access(&self, key: Node<K>) -> (Node<V>, Node<bool>);
}

/// degenerated DeviceCollection, K is the global invocation id in compute ctx
pub trait DeviceInvocation<V> {
  fn invocation_logic(&self, cx: ComputeCx) -> (Node<V>, Node<bool>);
}

pub struct DeviceParallelComputeCtx<'a> {
  pub gpu: &'a GPU,
}

pub trait DeviceParallelCompute<V> {
  fn build_invocation_access(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn FnOnce(&mut ShaderComputePipelineBuilder) -> Box<dyn DeviceInvocation<V>>>;
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
