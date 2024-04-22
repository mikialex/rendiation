#![feature(specialization)]

use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::sync::Arc;

use fast_hash_collection::*;
use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod io;
use io::*;
mod fork;
pub use fork::*;
mod radix_sort;
pub use radix_sort::*;
mod shuffle_move;
pub use shuffle_move::*;
mod map;
pub use map::*;
mod zip;
pub use zip::*;
mod prefix_scan;
pub use prefix_scan::*;
mod reduction;
pub use reduction::*;
mod histogram;
pub use histogram::*;
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

impl<T> DeviceInvocation<T> for Box<dyn DeviceInvocation<T>> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>) {
    (**self).invocation_logic(logic_global_id)
  }
}

pub trait DeviceInvocationExt<T>: DeviceInvocation<T> + 'static + Sized {
  fn into_boxed(self) -> Box<dyn DeviceInvocation<T>> {
    Box::new(self)
  }
  fn zip<U>(self, other: impl DeviceInvocation<U> + 'static) -> impl DeviceInvocation<(T, U)> {
    DeviceInvocationZip(self.into_boxed(), other.into_boxed())
  }
}
impl<T, X> DeviceInvocationExt<T> for X where X: DeviceInvocation<T> + 'static + Sized {}

pub struct AdhocInvocationResult<T>(pub T, pub Node<bool>);

impl<T: Copy> DeviceInvocation<T> for AdhocInvocationResult<T> {
  fn invocation_logic(&self, _: Node<Vec3<u32>>) -> (T, Node<bool>) {
    (self.0, self.1)
  }
}

pub trait DeviceInvocationComponent<T>: ShaderHashProviderAny {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>>;

  fn bind_input(&self, builder: &mut BindingBuilder);

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn dispatch_compute(&self, work_size: u32, cx: &mut DeviceParallelComputeCtx) {
    let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
    let pipeline = cx.get_or_create_compute_pipeline(self, |cx| {
      cx.config_work_group_size(workgroup_size);
      let invocation_source = self.build_shader(cx.0);

      let invocation_id = cx.local_invocation_id();
      let _ = invocation_source.invocation_logic(invocation_id);
    });

    let encoder = cx.gpu.create_encoder().compute_pass_scoped(|mut pass| {
      let mut bb = BindingBuilder::new_as_compute();
      self.bind_input(&mut bb);
      bb.setup_compute_pass(&mut pass, &cx.gpu.device, &pipeline);
      pass.dispatch_workgroups((work_size + workgroup_size - 1) / workgroup_size, 1, 1);
    });

    cx.gpu.submit_encoder(encoder);
  }
}

pub trait DeviceInvocationComponentExt<T>: DeviceInvocationComponent<T> {
  fn into_boxed(self) -> Box<dyn DeviceInvocationComponent<T>>;
}
impl<T, X> DeviceInvocationComponentExt<T> for X
where
  X: DeviceInvocationComponent<T> + 'static,
{
  fn into_boxed(self) -> Box<dyn DeviceInvocationComponent<T>> {
    Box::new(self)
  }
}

pub trait DeviceParallelCompute<T> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<T>>;

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
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    do_write_into_storage_buffer(self, cx)
  }
}

impl<T> DeviceParallelCompute<T> for Box<dyn DeviceParallelCompute<T>> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<T>> {
    (**self).execute_and_expose(cx)
  }

  fn work_size(&self) -> u32 {
    (**self).work_size()
  }
}
impl<T> DeviceParallelCompute<Node<T>> for Box<dyn DeviceParallelComputeIO<T>> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    (**self).execute_and_expose(cx)
  }

  fn work_size(&self) -> u32 {
    (**self).work_size()
  }
}
impl<T> DeviceParallelComputeIO<T> for Box<dyn DeviceParallelComputeIO<T>> {
  fn result_size(&self) -> u32 {
    (**self).work_size()
  }

  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    (**self).materialize_storage_buffer(cx)
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

  fn map<O: Copy + 'static>(self, mapper: impl Fn(T) -> O + 'static) -> DeviceMap<T, O> {
    DeviceMap {
      mapper: Arc::new(mapper),
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
  fn internal_materialize_storage_buffer(self) -> impl DeviceParallelComputeIO<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    WriteIntoStorageReadBackToDevice {
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

  fn shuffle_move(
    self,
    shuffle_idx: impl DeviceParallelCompute<Node<u32>> + 'static,
  ) -> impl DeviceParallelComputeIO<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    DataShuffleMovement {
      source: Box::new(self.zip(shuffle_idx)),
    }
  }

  fn workgroup_scope_reduction<S>(self, workgroup_size: u32) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: Std430 + ShaderSizedValueNodeType,
  {
    WorkGroupReduction::<T, S> {
      workgroup_size,
      reduction_logic: Default::default(),
      upstream: Box::new(self),
    }
    .internal_materialize_storage_buffer()
  }

  /// the total_work_size should not exceed first_stage_workgroup_size * second_stage_workgroup_size
  fn segmented_reduction<S>(
    self,
    first_stage_workgroup_size: u32,
    second_stage_workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: Std430,
  {
    assert!(self.work_size() <= first_stage_workgroup_size * second_stage_workgroup_size);

    self
      .workgroup_scope_reduction::<S>(first_stage_workgroup_size)
      .workgroup_scope_reduction::<S>(second_stage_workgroup_size)
  }

  /// perform workgroup scope histogram compute by workgroup level atomic array
  ///
  /// the entire histogram should be able to hold in workgroup
  /// workgroup_size should larger than histogram max
  fn workgroup_histogram<S>(self, workgroup_size: u32) -> impl DeviceParallelComputeIO<u32>
  where
    S: DeviceHistogramMappingLogic<Data = T> + 'static,
    T: Std430 + ShaderSizedValueNodeType,
  {
    WorkGroupHistogram::<T, S> {
      workgroup_size,
      logic: Default::default(),
      upstream: Box::new(self),
    }
    .internal_materialize_storage_buffer()
  }

  /// perform device scope histogram compute by workgroup level atomic array and global atomic array
  ///
  /// the entire work size should not exceed workgroup_privatization * 1024
  ///
  /// the entire histogram should be able to hold in workgroup
  /// workgroup_size should larger than histogram max
  fn histogram<S>(self, workgroup_privatization: u32) -> impl DeviceParallelComputeIO<u32>
  where
    S: DeviceHistogramMappingLogic<Data = T> + 'static,
    T: Std430 + ShaderSizedValueNodeType,
  {
    DeviceHistogram::<T, S> {
      workgroup_level: WorkGroupHistogram {
        workgroup_size: workgroup_privatization,
        logic: Default::default(),
        upstream: Box::new(self),
      },
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
    .internal_materialize_storage_buffer()
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

  fn device_radix_sort_naive<S>(
    self,
    per_pass_first_stage_workgroup_size: u32,
    per_pass_second_stage_workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceRadixSortKeyLogic<Data = T>,
    T: ShaderSizedValueNodeType + Std430,
  {
    device_radix_sort_naive::<T, S>(
      self,
      per_pass_first_stage_workgroup_size,
      per_pass_second_stage_workgroup_size,
    )
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
    source: &(impl ShaderHashProviderAny + ?Sized),
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

impl<T> ShaderHashProvider for Box<dyn DeviceInvocationComponent<T>> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline(hasher)
  }
}

impl<T> ShaderHashProviderAny for Box<dyn DeviceInvocationComponent<T>> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }
}
