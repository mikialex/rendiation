#![feature(specialization)]

use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
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
mod reduction;
pub use reduction::*;
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

/// This trait is to avoid all possible redundant storage buffer materialize but not requires
/// specialization. if the type impls DeviceParallelCompute<Node<T>>, it should impl this trait as
/// well.
pub trait DeviceParallelComputeIO<T>: DeviceParallelCompute<Node<T>> {
  /// if the material output size is different from work size(for example reduction), custom impl is
  /// required
  fn result_size(&self) -> u32 {
    self.work_size()
  }
  /// if the implementation already has materialized storage buffer, should provide it directly to
  /// avoid re-materialize cost
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    do_write_into_storage_buffer(self, cx, 256)
  }
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
impl<T> DeviceParallelCompute<Node<T>> for Box<dyn DeviceParallelComputeIO<T>> {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<Node<T>>> {
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
  fn stride_access_result(
    self,
    stride: impl Into<Vec3<u32>>,
  ) -> DeviceParallelComputeStrideRead<T> {
    DeviceParallelComputeStrideRead {
      source: Box::new(self),
      stride: stride.into(),
    }
  }

  fn map<O: Copy + 'static>(self, mapper: fn(T) -> O) -> DeviceMap<T, O> {
    DeviceMap {
      mapper,
      upstream: Box::new(self),
    }
  }

  fn zip<B: 'static>(
    self,
    other: impl DeviceParallelCompute<B> + 'static,
  ) -> DeviceParallelComputeZip<T, B> {
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

pub trait DeviceParallelComputeIOExt<T>: Sized + DeviceParallelComputeIO<T>
where
  T: ShaderSizedValueNodeType,
  Self: 'static,
{
  fn internal_materialize_storage_buffer(
    self,
    workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    WriteStorageReadBack {
      inner: Box::new(self),
      workgroup_size,
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
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: Std430 + ShaderSizedValueNodeType,
  {
    WorkGroupPrefixScanKoggeStone::<T, S> {
      workgroup_size,
      scan_logic: Default::default(),
      upstream: Box::new(self),
    }
    .internal_materialize_storage_buffer(workgroup_size)
  }

  /// the total_work_size should not exceed first_stage_workgroup_size * second_stage_workgroup_size
  fn segmented_prefix_scan_kogge_stone<S>(
    self,
    first_stage_workgroup_size: u32,
    second_stage_workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: Std430,
  {
    assert!(self.work_size() <= first_stage_workgroup_size * second_stage_workgroup_size);

    let per_workgroup_scanned = self
      .workgroup_scope_prefix_scan_kogge_stone::<S>(first_stage_workgroup_size)
      .into_forker();

    let block_wise_scanned = per_workgroup_scanned
      .clone()
      .stride_access_result((first_stage_workgroup_size, 1, 1))
      .workgroup_scope_prefix_scan_kogge_stone::<S>(second_stage_workgroup_size);

    block_wise_scanned
      .zip(per_workgroup_scanned)
      .map(|(block_scan, workgroup_scan)| S::combine(block_scan, workgroup_scan))
  }
}

impl<X, T> DeviceParallelComputeIOExt<T> for X
where
  X: Sized + DeviceParallelComputeIO<T> + 'static,
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
