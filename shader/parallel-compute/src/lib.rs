use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::sync::Arc;

use derive_where::*;
use dyn_clone::DynClone;
use hook::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod abstract_component;
pub use abstract_component::*;
mod abstract_invocation;
pub use abstract_invocation::*;
mod ctx;
pub use ctx::*;
mod io;
pub use io::*;
mod radix_sort;
pub use radix_sort::*;
mod stream_compaction;
pub use stream_compaction::*;
mod shuffle_move;
pub use shuffle_move::*;
mod map;
pub use map::*;
mod access_behavior;
pub use access_behavior::*;
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

pub trait DeviceInvocationComponentExt<T>: ComputeComponent<T> {
  fn into_boxed(self) -> Box<dyn ComputeComponent<T>>;
}
impl<T, X> DeviceInvocationComponentExt<T> for X
where
  X: ComputeComponent<T> + 'static,
{
  fn into_boxed(self) -> Box<dyn ComputeComponent<T>> {
    Box::new(self)
  }
}

pub trait DeviceParallelComputeExt<T>: Sized + ComputeComponent<T>
where
  Self: 'static,
  T: 'static,
{
  /// offset should smaller than stride
  fn stride_reduce_result(self, stride: u32) -> DeviceParallelComputeStrideRead<T> {
    self.stride_access_result(stride, true)
  }

  /// offset should smaller than stride
  fn stride_expand_result(self, stride: u32) -> DeviceParallelComputeStrideRead<T> {
    self.stride_access_result(stride, false)
  }

  /// offset should smaller than stride
  fn stride_access_result(self, stride: u32, reduce: bool) -> DeviceParallelComputeStrideRead<T> {
    assert!(stride > 0);
    DeviceParallelComputeStrideRead {
      source: Box::new(self),
      stride,
      reduce,
    }
  }

  fn map<O: Copy + 'static, F: Fn(T) -> O + 'static>(self, mapper: F) -> DeviceMapCompute<T, O> {
    struct TypeHash(std::any::TypeId);
    impl ShaderHashProvider for TypeHash {
      shader_hash_type_id! {}
      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.0.hash(hasher);
      }
    }

    DeviceMapCompute {
      mapper: Arc::new(mapper),
      upstream: Box::new(self),
      mapper_extra_hasher: Arc::new(TypeHash(std::any::TypeId::of::<F>())),
    }
  }

  fn map_with_extra_hasher<O: Copy + 'static, F: Fn(T) -> O + 'static>(
    self,
    mapper: F,
    hasher: impl ShaderHashProvider + 'static,
  ) -> DeviceMapCompute<T, O> {
    DeviceMapCompute {
      mapper: Arc::new(mapper),
      upstream: Box::new(self),
      mapper_extra_hasher: Arc::new(hasher),
    }
  }

  /// if map closure capture values, values should be hashed by hasher
  fn map_with_id_provided<O: Copy + 'static>(
    self,
    mapper: impl Fn(T) -> O + 'static,
    hasher: impl ShaderHashProvider + 'static,
  ) -> DeviceMapCompute<T, O> {
    DeviceMapCompute {
      mapper: Arc::new(mapper),
      upstream: Box::new(self),
      mapper_extra_hasher: Arc::new(hasher),
    }
  }

  fn zip<B: 'static>(self, other: impl ComputeComponent<B> + 'static) -> DeviceComputeZip<T, B> {
    DeviceComputeZip {
      source_a: Box::new(self),
      source_b: Box::new(other),
    }
  }
}

impl<X, T> DeviceParallelComputeExt<T> for X
where
  X: Sized + ComputeComponent<T> + 'static,
  T: 'static,
{
}

#[allow(async_fn_in_trait)]
pub trait DeviceParallelComputeIOExt<T>: Sized + ComputeComponentIO<T>
where
  T: ShaderSizedValueNodeType + Std430 + Debug,
  Self: 'static,
{
  async fn run_test(&self, cx: &mut DeviceParallelComputeCtx<'_>, expect: &[T])
  where
    T: Debug + PartialEq,
  {
    self.run_test_with_size_test(cx, expect, None).await
  }

  async fn run_test_with_size_test(
    &self,
    cx: &mut DeviceParallelComputeCtx<'_>,
    expect: &[T],
    expect_size: Option<Vec3<u32>>,
  ) where
    T: Debug + PartialEq,
  {
    fn check<T: PartialEq + Debug>(expect: &[T], result: &[T]) {
      if expect != result {
        panic!(
          "wrong result:  {:?} \n != \nexpect result: {:?}",
          result, expect
        )
      }
    }

    cx.force_indirect_dispatch = false;
    let (_, size, result) = self.read_back_host(cx).await.unwrap();
    check(expect, &result);
    if let (Some(size), Some(expect_size)) = (size, expect_size) {
      assert_eq!(size, expect_size);
    }

    cx.gpu.device.clear_resource_cache(); // todo , fixme

    cx.force_indirect_dispatch = true;
    let (_, size, result) = self.read_back_host(cx).await.unwrap();

    check(expect, &result);
    if let (Some(size), Some(expect_size)) = (size, expect_size) {
      assert_eq!(size, expect_size);
    }
  }

  async fn read_back_host(
    &self,
    cx: &mut DeviceParallelComputeCtx<'_>,
  ) -> Result<(DeviceMaterializeResult<T>, Option<Vec3<u32>>, Vec<T>), BufferAsyncError> {
    let output = self.materialize_storage_buffer(cx);
    cx.flush_pass();
    let result = cx.encoder.read_buffer(&cx.gpu.device, &output.buffer);
    let size_result = output
      .size
      .as_ref()
      .map(|size| cx.encoder.read_buffer(&cx.gpu.device, size));
    cx.submit_recorded_work_and_continue();
    let result = result.await;
    let size_result = if let Some(size_result) = size_result {
      let size = size_result.await?;
      let size = *from_bytes::<Vec4<u32>>(&size.read_raw());
      Some(size.xyz())
    } else {
      None
    };

    result.map(|r| {
      (
        output,
        size_result,
        <[T]>::from_bytes_into_boxed(&r.read_raw()).into_vec(),
      )
    })
  }

  fn debug_log(&self, label: &'static str, cx: &mut DeviceParallelComputeCtx)
  where
    T: std::fmt::Debug,
  {
    let (_, size, host_result) = pollster::block_on(self.read_back_host(cx)).unwrap();

    println!("{} content is: {:?}", label, host_result);
    if let Some(size) = size {
      println!("{} has device size: {}", label, size);
    }
  }

  fn shuffle_move(
    self,
    shuffle_idx: impl ComputeComponent<(Node<u32>, Node<bool>)> + 'static,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T> {
    let output = cx.use_rw_storage_buffer(self.result_size() as usize);
    let write = ShuffleWrite {
      input: Box::new(
        self
          .zip(shuffle_idx)
          .map(|(v, (id, should))| (v, id, should)),
      ),
      output,
    };

    // should size be the atomic max of the shuffle destination?
    let size = write.dispatch_compute(cx);
    DeviceMaterializeResult {
      buffer: write.output.into_readonly_view(),
      size,
    }
  }

  fn workgroup_scope_reduction<S>(
    self,
    workgroup_size: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    WorkGroupReductionCompute::<T, S> {
      workgroup_size,
      reduction_logic: Default::default(),
      upstream: Box::new(self),
    }
    .materialize_storage_buffer(cx)
  }

  /// the total_work_size should not exceed first_stage_workgroup_size * second_stage_workgroup_size
  fn segmented_reduction<S>(
    self,
    first_stage_workgroup_size: u32,
    second_stage_workgroup_size: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> impl ComputeComponentIO<T> + 'static
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    // assert!(self.max_work_size() <= first_stage_workgroup_size * second_stage_workgroup_size);

    self
      .workgroup_scope_reduction::<S>(first_stage_workgroup_size, cx)
      .stride_reduce_result(first_stage_workgroup_size)
      .workgroup_scope_reduction::<S>(second_stage_workgroup_size, cx)
      .stride_reduce_result(second_stage_workgroup_size)
  }

  /// perform workgroup scope histogram compute by workgroup level atomic array
  ///
  /// the entire histogram should be able to hold in workgroup
  /// workgroup_size should larger than histogram max
  fn workgroup_histogram<S>(
    self,
    workgroup_size: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<u32>
  where
    S: DeviceHistogramMappingLogic<Data = T> + 'static,
  {
    WorkGroupHistogramCompute::<T, S> {
      workgroup_size,
      histogram_logic: Default::default(),
      upstream: Box::new(self),
    }
    .materialize_storage_buffer(cx)
  }

  /// perform device scope histogram compute by workgroup level atomic array and global atomic array
  ///
  /// the entire work size should not exceed workgroup_privatization * 1024
  ///
  /// the entire histogram should be able to hold in workgroup
  /// workgroup_size should larger than histogram max
  fn histogram<S>(
    self,
    workgroup_privatization: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<u32>
  where
    S: DeviceHistogramMappingLogic<Data = T> + 'static,
  {
    assert!(S::MAX <= workgroup_privatization);

    let init = ZeroedArrayByArrayLength(S::MAX as usize);
    let result = create_gpu_read_write_storage(init, &cx.gpu.device);

    DeviceHistogramCompute::<T, S> {
      workgroup_level: WorkGroupHistogramCompute {
        workgroup_size: workgroup_privatization,
        histogram_logic: Default::default(),
        upstream: Box::new(self),
      },
      result,
    }
    .materialize_storage_buffer(cx)
  }

  fn custom_access(
    self,
    behavior: impl InvocationAccessBehavior<T> + 'static + Hash,
  ) -> impl ComputeComponent<Node<T>> {
    DeviceParallelComputeCustomInvocationBehavior {
      source: Box::new(self),
      behavior,
    }
  }

  fn offset_access(
    self,
    offset: i32,
    ob: OutBoundsBehavior<T>,
    size_expand: u32,
  ) -> impl ComputeComponentIO<T> {
    DeviceParallelComputeCustomInvocationBehavior {
      source: Box::new(self),
      behavior: DeviceInvocationOffset {
        offset,
        ob,
        size_expand,
      },
    }
  }

  fn stream_compaction(
    self,
    filter: impl ComputeComponentIO<bool> + 'static,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T> {
    stream_compaction(Box::new(self), Box::new(filter), cx)
  }

  /// this is not very useful but sometimes feel handy, so I will keep it here
  fn stream_compaction_self_filter(
    self,
    filter: impl Fn(Node<T>) -> Node<bool> + 'static,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    Self: Clone,
  {
    let mask = self.clone().map(filter);
    self.stream_compaction(mask, cx)
  }

  fn workgroup_scope_prefix_scan_kogge_stone<S>(
    self,
    workgroup_size: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    WorkGroupPrefixScanKoggeStoneCompute::<T, S> {
      workgroup_size,
      scan_logic: Default::default(),
      upstream: Box::new(self),
    }
    .materialize_storage_buffer(cx)
  }

  /// the scan is inclusive, using make_global_scan_exclusive to convert it to exclusive
  ///
  /// the total_work_size must not exceed first_stage_workgroup_size * second_stage_workgroup_size
  fn segmented_prefix_scan_kogge_stone<S>(
    self,
    first_stage_workgroup_size: u32,
    second_stage_workgroup_size: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    // todo, impl another way to check if it's ok to run compute
    // assert!(self.max_work_size() <= first_stage_workgroup_size * second_stage_workgroup_size);

    let per_workgroup_scanned =
      self.workgroup_scope_prefix_scan_kogge_stone::<S>(first_stage_workgroup_size, cx);

    let block_wise_scanned = per_workgroup_scanned
      .clone()
      .offset_access(
        first_stage_workgroup_size as i32 - 1,
        OutBoundsBehavior::ClampBorder,
        0,
      )
      .stride_reduce_result(first_stage_workgroup_size)
      .workgroup_scope_prefix_scan_kogge_stone::<S>(second_stage_workgroup_size, cx)
      .make_global_scan_exclusive::<S>()
      .stride_expand_result(first_stage_workgroup_size);

    per_workgroup_scanned
      .zip(block_wise_scanned)
      .map(|(block_scan, workgroup_scan)| S::combine(block_scan, workgroup_scan))
      .materialize_storage_buffer(cx) // todo,remove and  fix compatibility issue
  }

  /// should logically be used after global inclusive scan
  fn make_global_scan_exclusive<S>(self) -> impl ComputeComponentIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    self.offset_access(-1, OutBoundsBehavior::from_const(|| S::identity()), 1)
  }

  fn device_radix_sort_naive<S>(
    self,
    per_pass_first_stage_workgroup_size: u32,
    per_pass_second_stage_workgroup_size: u32,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn ComputeComponentIO<T>>
  where
    S: DeviceRadixSortKeyLogic<Data = T>,
  {
    device_radix_sort_naive::<T, S>(
      self,
      per_pass_first_stage_workgroup_size,
      per_pass_second_stage_workgroup_size,
      cx,
    )
  }
}

impl<X, T> DeviceParallelComputeIOExt<T> for X
where
  X: Sized + ComputeComponentIO<T> + 'static,
  T: ShaderSizedValueNodeType + Std430 + Debug,
{
}
