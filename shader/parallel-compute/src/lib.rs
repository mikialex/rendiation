#![feature(specialization)]

use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use fast_hash_collection::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod io;
use io::*;
mod map;
pub use map::*;
mod zip;
pub use zip::*;
mod prefix_scan;
pub use prefix_scan::*;
mod stride_read;
pub use stride_read::*;

/// pure shader structures
pub trait DeviceCollection<K, T> {
  /// should not contain any side effects
  fn device_access(&self, key: Node<K>) -> (T, Node<bool>);
}

/// degenerated DeviceCollection, K is the global invocation id in compute ctx
pub trait DeviceInvocation<T> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>);
}

pub struct AdhocInvocationResult<T>(pub T, pub Node<bool>);

impl<T: Copy> DeviceInvocation<T> for AdhocInvocationResult<T> {
  fn invocation_logic(&self, _: Node<Vec3<u32>>) -> (T, Node<bool>) {
    (self.0, self.1)
  }
}

pub trait DeviceInvocationBuilder<T>: ShaderHashProviderAny {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>>;

  fn bind_input(&self, builder: &mut BindingBuilder);
}

pub trait DeviceParallelCompute<T> {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<T>>;

  // the total invocation count, this is useful to get linear results back
  fn work_size(&self) -> u32;
}

impl<T> DeviceParallelCompute<T> for Box<dyn DeviceParallelCompute<T>> {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<T>> {
    (**self).compute_result(cx)
  }

  fn work_size(&self) -> u32 {
    (**self).work_size()
  }
}

pub trait DeviceParallelComputeExt<T>: Sized + DeviceParallelCompute<T>
where
  Self: 'static,
  T: 'static,
{
  fn stride_access_result(self, stride: impl Into<Vec3<u32>>) -> impl DeviceParallelCompute<T> {
    DeviceParallelComputeStrideRead {
      source: Box::new(self),
      stride: stride.into(),
    }
  }

  fn map<O: Copy + 'static>(self, mapper: fn(T) -> O) -> impl DeviceParallelCompute<O> {
    DeviceMap {
      mapper,
      upstream: Box::new(self),
    }
  }

  fn zip<B: 'static>(
    self,
    other: impl DeviceParallelCompute<B> + 'static,
  ) -> impl DeviceParallelCompute<(T, B)> {
    DeviceParallelComputeZip {
      source_a: Box::new(self),
      source_b: Box::new(other),
    }
  }
}

impl<X, T> DeviceParallelComputeExt<T> for X
where
  X: Sized + DeviceParallelCompute<T> + 'static,
  T: 'static,
{
}

pub trait DeviceParallelComputeNodeExt<T>: Sized + DeviceParallelCompute<Node<T>>
where
  T: ShaderSizedValueNodeType,
  Self: 'static,
{
  fn write_into_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferDataView<[T]>
  where
    Self: Sized,
    T: Std430 + ShaderSizedValueNodeType,
  {
    write_into_storage_buffer(self, cx)
  }

  fn materialize_storage_buffer(self) -> impl DeviceParallelCompute<Node<T>>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    WriteStorageReadBack {
      inner: Box::new(self),
    }
  }

  fn into_forker(self) -> ComputeResultForkerInstance<T>
  where
    T: Std430,
  {
    ComputeResultForkerInstance {
      upstream: Arc::new(ComputeResultForker {
        inner: Box::new(self),
        children: Default::default(),
      }),
      result: Default::default(),
    }
  }

  fn workgroup_scope_prefix_scan_kogge_stone<S>(
    self,
    workgroup_size: u32,
  ) -> impl DeviceParallelCompute<Node<T>>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: Std430 + ShaderSizedValueNodeType,
  {
    WorkGroupPrefixScanKoggeStone::<T, S> {
      workgroup_size,
      scan_logic: Default::default(),
      upstream: Box::new(self),
    }
    .materialize_storage_buffer()
  }

  /// the total_work_size should not exceed first_stage_workgroup_size * 1024
  fn segmented_prefix_scan_kogge_stone<S>(
    self,
    first_stage_workgroup_size: u32,
  ) -> impl DeviceParallelCompute<Node<T>>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: Std430,
  {
    assert!(self.work_size() <= first_stage_workgroup_size * 1024);

    let per_workgroup_scanned = self
      .workgroup_scope_prefix_scan_kogge_stone::<S>(first_stage_workgroup_size)
      .into_forker();

    let block_wise_scanned = per_workgroup_scanned
      .clone()
      .stride_access_result((first_stage_workgroup_size, 1, 1))
      .workgroup_scope_prefix_scan_kogge_stone::<S>(1024); // should this be configurable?

    block_wise_scanned
      .zip(per_workgroup_scanned)
      .map(|(block_scan, workgroup_scan)| S::combine(block_scan, workgroup_scan))
  }
}

impl<X, T> DeviceParallelComputeNodeExt<T> for X
where
  X: Sized + DeviceParallelCompute<Node<T>> + 'static,
  T: ShaderSizedValueNodeType,
{
}

pub struct DeviceParallelComputeCtx<'a> {
  pub gpu: &'a GPU,
  pub compute_pipeline_cache: FastHashMap<u64, GPUComputePipeline>,
}

impl<'a> DeviceParallelComputeCtx<'a> {
  pub fn get_or_create_compute_pipeline(
    &mut self,
    source: &impl ShaderHashProviderAny,
    creator: impl FnOnce(&mut ComputeCx),
  ) -> GPUComputePipeline {
    let mut hasher = PipelineHasher::default();
    source.hash_pipeline_with_type_info(&mut hasher);
    let hash = hasher.finish();

    self
      .compute_pipeline_cache
      .entry(hash)
      .or_insert_with(|| {
        compute_shader_builder()
          .entry(|cx| {
            creator(cx);
          })
          .create_compute_pipeline(self.gpu)
          .unwrap()
      })
      .clone()
  }
}

impl<T> ShaderHashProvider for Box<dyn DeviceInvocationBuilder<T>> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline(hasher)
  }
}

impl<T> ShaderHashProviderAny for Box<dyn DeviceInvocationBuilder<T>> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }
}
