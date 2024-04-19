#![feature(specialization)]

use std::hash::Hasher;

use fast_hash_collection::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod io;
use io::*;
mod prefix_scan;
pub use prefix_scan::*;

/// pure shader structures
pub trait DeviceCollection<K, T> {
  /// should not contain any side effects
  fn device_access(&self, key: Node<K>) -> (Node<T>, Node<bool>);
}

/// degenerated DeviceCollection, K is the global invocation id in compute ctx
pub trait DeviceInvocation<T> {
  fn invocation_logic(&self, cx: &mut ComputeCx) -> (Node<T>, Node<bool>);
}

pub struct AdhocInvocationResult<T>(pub Node<T>, pub Node<bool>);

impl<T: ShaderNodeType> DeviceInvocation<T> for AdhocInvocationResult<T> {
  fn invocation_logic(&self, _: &mut ComputeCx) -> (Node<T>, Node<bool>) {
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

pub trait DeviceParallelCompute<T>: 'static {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<T>>;

  // the total invocation count, this is useful to get linear results back
  fn work_size(&self) -> u32;
}

impl<T: ShaderSizedValueNodeType> DeviceParallelCompute<T> for Box<dyn DeviceParallelCompute<T>> {
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
  T: ShaderSizedValueNodeType,
{
  fn write_into_storage_buffer(
    self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    write_into_storage_buffer(&self, cx)
  }

  fn write_storage_read_back(self) -> impl DeviceParallelCompute<T>
  where
    T: Std430,
  {
    WriteStorageReadBack {
      inner: Box::new(self),
    }
  }

  fn workgroup_scope_prefix_scan<S>(self, workgroup_size: u32) -> impl DeviceParallelCompute<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
    T: ShaderSizedValueNodeType + Std430,
  {
    WorkGroupPrefixScan::<T, S> {
      workgroup_size,
      scan_logic: Default::default(),
      upstream: Box::new(self),
    }
    .write_storage_read_back()
  }
}

impl<X, T> DeviceParallelComputeExt<T> for X
where
  X: Sized + DeviceParallelCompute<T>,
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
