#![feature(let_chains)]

use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::sync::Arc;

use derivative::Derivative;
use dyn_clone::DynClone;
use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod io;
pub use io::*;
mod fork;
pub use fork::*;

mod radix_sort;
pub use radix_sort::*;
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

/// abstract device invocation. the invocation cost should only exist if user has called
///  `invocation_logic`, as well as invocation_size.
pub trait DeviceInvocation<T> {
  // todo, we should separate check and access in different fn to avoid unnecessary check;
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>);

  fn invocation_size(&self) -> Node<Vec3<u32>>;

  fn end_point(&self) -> T {
    let clamp_target = self.invocation_size() - val(Vec3::one());
    self.invocation_logic(clamp_target).0
  }

  fn start_point(&self) -> T {
    self.invocation_logic(val(Vec3::zero())).0
  }
}

impl<T> DeviceInvocation<T> for Box<dyn DeviceInvocation<T>> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>) {
    (**self).invocation_logic(logic_global_id)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (**self).invocation_size()
  }
}

pub trait DeviceInvocationExt<T>: DeviceInvocation<T> + 'static + Sized {
  fn into_boxed(self) -> Box<dyn DeviceInvocation<T>> {
    Box::new(self)
  }

  fn zip<U>(self, other: impl DeviceInvocation<U> + 'static) -> DeviceInvocationZip<T, U> {
    DeviceInvocationZip(self.into_boxed(), other.into_boxed())
  }

  fn adhoc_invoke_with_self_size<R>(
    self,
    r: impl Fn(&Self, Node<Vec3<u32>>) -> (R, Node<bool>) + 'static,
  ) -> impl DeviceInvocation<R>
  where
    R: Copy,
  {
    AdhocInvocationResult {
      upstream: self,
      phantom: PhantomData,
      compute: Box::new(r),
    }
  }
}
impl<T, X> DeviceInvocationExt<T> for X where X: DeviceInvocation<T> + 'static + Sized {}

pub struct RealAdhocInvocationResult<S, R> {
  pub inner: S,
  pub compute: Box<dyn Fn(&S, Node<Vec3<u32>>) -> (R, Node<bool>)>,
  pub size: Box<dyn Fn(&S) -> Node<Vec3<u32>>>,
}

impl<S, R> DeviceInvocation<R> for RealAdhocInvocationResult<S, R> {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (R, Node<bool>) {
    (self.compute)(&self.inner, id)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.size)(&self.inner)
  }
}

/// i think this is a mistake
pub struct AdhocInvocationResult<S, T, R> {
  upstream: S,
  phantom: PhantomData<T>,
  compute: Box<dyn Fn(&S, Node<Vec3<u32>>) -> (R, Node<bool>)>,
}

impl<S: DeviceInvocation<T>, T, R> DeviceInvocation<R> for AdhocInvocationResult<S, T, R> {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (R, Node<bool>) {
    (self.compute)(&self.upstream, id)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.upstream.invocation_size()
  }
}

pub fn compute_dispatch_size(work_size: u32, workgroup_size: u32) -> u32 {
  (work_size + workgroup_size - 1) / workgroup_size
}
pub fn device_compute_dispatch_size(work_size: Node<u32>, workgroup_size: Node<u32>) -> Node<u32> {
  (work_size + workgroup_size - val(1)) / workgroup_size
}

pub trait DeviceInvocationComponent<T>: ShaderHashProvider {
  fn work_size(&self) -> Option<u32>;

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>>;

  fn bind_input(&self, builder: &mut BindingBuilder);

  fn requested_workgroup_size(&self) -> Option<u32>;

  fn dispatch_compute(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Option<StorageBufferReadOnlyDataView<Vec3<u32>>> {
    if !cx.force_indirect_dispatch
      && let Some(work_size) = self.work_size()
    {
      let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
      self.prepare_main_pass(cx);
      cx.record_pass(|pass, _| {
        pass.dispatch_workgroups(compute_dispatch_size(work_size, workgroup_size), 1, 1);
      });
      None
    } else {
      let (indirect_dispatch_size, indirect_work_size) = self.compute_work_size(cx);
      self.prepare_main_pass(cx);
      cx.record_pass(|pass, _| {
        pass.dispatch_workgroups_indirect_owned(&indirect_dispatch_size);
      });
      Some(indirect_work_size.into_readonly_view())
    }
  }

  fn prepare_main_pass(&self, cx: &mut DeviceParallelComputeCtx) {
    let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
    let main_pipeline = cx.get_or_create_compute_pipeline(self, |cx| {
      cx.config_work_group_size(workgroup_size);
      let invocation_source = self.build_shader(cx.0);

      let invocation_id = cx.local_invocation_id();
      let _ = invocation_source.invocation_logic(invocation_id);
    });
    cx.record_pass(|pass, _| {
      let mut bb = BindingBuilder::new_as_compute();
      self.bind_input(&mut bb);
      bb.setup_compute_pass(pass, &cx.gpu.device, &main_pipeline);
    });
  }

  fn compute_work_size(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> (
    StorageBufferDataView<DispatchIndirectArgsStorage>,
    StorageBufferDataView<Vec3<u32>>,
  ) {
    struct SizeWriter<'a, T: ?Sized>(&'a T);
    impl<'a, T: ShaderHashProvider + ?Sized> ShaderHashProvider for SizeWriter<'a, T> {
      fn hash_type_info(&self, hasher: &mut PipelineHasher) {
        self.0.hash_type_info(hasher)
      }

      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        0.hash(hasher);
        self.0.hash_pipeline(hasher)
      }
    }

    let size_output = cx.gpu.device.make_indirect_dispatch_size_buffer();
    let work_size_output =
      create_gpu_readonly_storage(&Vec3::<u32>::zero(), &cx.gpu.device).into_rw_view();

    // requested_workgroup_size should always be respected
    let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
    let workgroup_size_buffer = create_gpu_readonly_storage(&workgroup_size, &cx.gpu.device);

    let pipeline = cx.get_or_create_compute_pipeline(&SizeWriter(self), |cx| {
      cx.config_work_group_size(workgroup_size);

      let size_output = cx.bind_by(&size_output);
      let work_size_output = cx.bind_by(&work_size_output);
      let workgroup_size = cx.bind_by(&workgroup_size_buffer);

      let size = self.build_shader(cx.0).invocation_size();

      work_size_output.store(size);

      let size = ENode::<DispatchIndirectArgsStorage> {
        x: device_compute_dispatch_size(size.x(), workgroup_size.load()),
        y: size.y(),
        z: size.z(),
      }
      .construct();

      size_output.store(size);
    });

    cx.record_pass(|pass, _| {
      let mut bb = BindingBuilder::new_as_compute()
        .with_bind(&size_output)
        .with_bind(&work_size_output)
        .with_bind(&workgroup_size_buffer);
      self.bind_input(&mut bb);

      bb.setup_compute_pass(pass, &cx.gpu.device, &pipeline);
      pass.dispatch_workgroups(1, 1, 1);
    });

    (size_output, work_size_output)
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

/// The top level composable trait for parallel compute.
///
/// Note that the clone is implemented by duplicating upstream work, if you want to reuse the work
/// by materialize and share the result, you should using the fork operator, instead of call clone
/// after internal_materialize_storage_buffer
pub trait DeviceParallelCompute<T>: DynClone {
  /// The main logic is expressed in this fn call. The implementation could do multiple dispatch in
  /// this function, just to prepare all the necessary data the final exposing step required
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<T>>;

  /// if the material output size is different from execute_and_expose's work size(for example reduction),
  /// custom impl or multi dispatch is required
  fn result_size(&self) -> u32;
}

/// This trait is to avoid all possible redundant storage buffer materialize but not requires
/// specialization. if the type impls DeviceParallelCompute<Node<T>>, it should impl this trait as
/// well.
pub trait DeviceParallelComputeIO<T>: DeviceParallelCompute<Node<T>> {
  /// if the implementation already has materialized storage buffer, should provide it directly to
  /// avoid re-materialize cost, the user should not mutate the materialized result
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
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

  fn result_size(&self) -> u32 {
    (**self).result_size()
  }
}
impl<T> Clone for Box<dyn DeviceParallelCompute<T>> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}
impl<T> DeviceParallelCompute<Node<T>> for Box<dyn DeviceParallelComputeIO<T>> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    (**self).execute_and_expose(cx)
  }
  fn result_size(&self) -> u32 {
    (**self).result_size()
  }
}
impl<T> Clone for Box<dyn DeviceParallelComputeIO<T>> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}
impl<T> DeviceParallelComputeIO<T> for Box<dyn DeviceParallelComputeIO<T>> {
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
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

  fn map<O: Copy + 'static>(self, mapper: impl Fn(T) -> O + 'static) -> DeviceMap<T, O> {
    DeviceMap {
      mapper: Arc::new(mapper),
      upstream: Box::new(self),
      mapper_extra_hasher: Arc::new(()),
    }
  }

  /// if map closure capture values, values should be hashed by hasher
  fn map_with_id_provided<O: Copy + 'static>(
    self,
    mapper: impl Fn(T) -> O + 'static,
    hasher: impl ShaderHashProvider + 'static,
  ) -> DeviceMap<T, O> {
    DeviceMap {
      mapper: Arc::new(mapper),
      upstream: Box::new(self),
      mapper_extra_hasher: Arc::new(hasher),
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

#[allow(async_fn_in_trait)]
pub trait DeviceParallelComputeIOExt<T>: Sized + DeviceParallelComputeIO<T>
where
  T: ShaderSizedValueNodeType + Std430 + Debug,
  Self: 'static,
{
  async fn run_test(&self, expect: &[T])
  where
    T: Debug + PartialEq,
  {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    let mut cx = DeviceParallelComputeCtx::new(&gpu);

    fn check<T: PartialEq + Debug>(expect: &[T], result: &[T]) {
      if expect != result {
        panic!(
          "wrong result:  {:?} \n != \nexpect result: {:?}",
          result, expect
        )
      }
    }

    cx.force_indirect_dispatch = false;
    let (_, result) = self.read_back_host(&mut cx).await.unwrap();
    check(expect, &result);

    // cx.force_indirect_dispatch = true;
    // let (_, result) = self.read_back_host(&mut cx).await.unwrap();
    // check(expect, &result);
  }

  // todo, read back real size on device
  async fn read_back_host(
    &self,
    cx: &mut DeviceParallelComputeCtx<'_>,
  ) -> Result<(DeviceMaterializeResult<T>, Vec<T>), BufferAsyncError> {
    let output = self.materialize_storage_buffer(cx);
    cx.flush_pass();
    let result = cx.encoder.read_buffer(&cx.gpu.device, &output.buffer);
    cx.submit_recorded_work_and_continue();
    let result = result.await;

    result.map(|r| {
      (
        output,
        <[T]>::from_bytes_into_boxed(&r.read_raw()).into_vec(),
      )
    })
  }

  fn debug_log(self, label: &'static str) -> impl DeviceParallelComputeIO<T>
  where
    T: std::fmt::Debug,
  {
    DeviceParallelComputeIODebug {
      inner: Box::new(self),
      label,
    }
  }

  fn internal_materialize_storage_buffer(self) -> impl DeviceParallelComputeIO<T> {
    WriteIntoStorageReadBackToDevice {
      inner: Box::new(self),
    }
  }

  fn into_forker(self) -> ComputeResultForkerInstance<T> {
    ComputeResultForkerInstance::from_upstream(Box::new(self))
  }

  fn shuffle_move(
    self,
    shuffle_idx: impl DeviceParallelCompute<(Node<u32>, Node<bool>)> + 'static,
  ) -> impl DeviceParallelComputeIO<T> {
    DataShuffleMovement {
      source: Box::new(
        self
          .zip(shuffle_idx)
          .map(|(v, (id, should))| (v, id, should)),
      ),
    }
  }

  fn workgroup_scope_reduction<S>(self, workgroup_size: u32) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
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
  {
    // assert!(self.max_work_size() <= first_stage_workgroup_size * second_stage_workgroup_size);

    self
      .workgroup_scope_reduction::<S>(first_stage_workgroup_size)
      .stride_reduce_result(first_stage_workgroup_size)
      .workgroup_scope_reduction::<S>(second_stage_workgroup_size)
      .stride_reduce_result(second_stage_workgroup_size)
  }

  /// perform workgroup scope histogram compute by workgroup level atomic array
  ///
  /// the entire histogram should be able to hold in workgroup
  /// workgroup_size should larger than histogram max
  fn workgroup_histogram<S>(self, workgroup_size: u32) -> impl DeviceParallelComputeIO<u32>
  where
    S: DeviceHistogramMappingLogic<Data = T> + 'static,
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
  {
    assert!(S::MAX <= workgroup_privatization);
    DeviceHistogram::<T, S> {
      workgroup_level: WorkGroupHistogram {
        workgroup_size: workgroup_privatization,
        logic: Default::default(),
        upstream: Box::new(self),
      },
    }
  }

  fn custom_access(
    self,
    behavior: impl InvocationAccessBehavior<T> + 'static + Hash,
  ) -> impl DeviceParallelComputeIO<T> {
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
  ) -> impl DeviceParallelComputeIO<T> {
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
    filter: impl DeviceParallelComputeIO<bool> + 'static,
  ) -> impl DeviceParallelComputeIO<T> {
    let filter: Box<dyn DeviceParallelComputeIO<bool>> = Box::new(filter);
    let write_target_positions = filter
      .clone()
      .map(|v| v.select(1_u32, 0))
      .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(64, 64);

    self.shuffle_move(write_target_positions.zip(filter))
  }

  fn workgroup_scope_prefix_scan_kogge_stone<S>(
    self,
    workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    WorkGroupPrefixScanKoggeStone::<T, S> {
      workgroup_size,
      scan_logic: Default::default(),
      upstream: Box::new(self),
    }
    .internal_materialize_storage_buffer()
  }

  /// todo, support exclusive
  /// the total_work_size should not exceed first_stage_workgroup_size * second_stage_workgroup_size
  fn segmented_prefix_scan_kogge_stone<S>(
    self,
    first_stage_workgroup_size: u32,
    second_stage_workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    // todo, impl another way to check if it's ok to run compute
    // assert!(self.max_work_size() <= first_stage_workgroup_size * second_stage_workgroup_size);

    let per_workgroup_scanned = self
      .workgroup_scope_prefix_scan_kogge_stone::<S>(first_stage_workgroup_size)
      .into_forker();

    let block_wise_scanned = per_workgroup_scanned
      .clone()
      .offset_access(
        first_stage_workgroup_size as i32 - 1,
        OutBoundsBehavior::ClampBorder,
        0,
      )
      .stride_reduce_result(first_stage_workgroup_size)
      .workgroup_scope_prefix_scan_kogge_stone::<S>(second_stage_workgroup_size)
      .make_global_scan_exclusive::<S>()
      .stride_expand_result(first_stage_workgroup_size);

    per_workgroup_scanned
      .zip(block_wise_scanned)
      .map(|(block_scan, workgroup_scan)| S::combine(block_scan, workgroup_scan))
      .internal_materialize_storage_buffer() // todo,remove and  fix compatibility issue
  }

  /// should logically used after global inclusive scan
  fn make_global_scan_exclusive<S>(self) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceMonoidLogic<Data = T> + 'static,
  {
    self.offset_access(-1, OutBoundsBehavior::from_const(|| S::identity()), 1)
  }

  fn device_radix_sort_naive<S>(
    self,
    per_pass_first_stage_workgroup_size: u32,
    per_pass_second_stage_workgroup_size: u32,
  ) -> impl DeviceParallelComputeIO<T>
  where
    S: DeviceRadixSortKeyLogic<Data = T>,
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
  T: ShaderSizedValueNodeType + Std430 + Debug,
{
}

pub struct DeviceParallelComputeCtx<'a> {
  pub gpu: &'a GPU,
  encoder: GPUCommandEncoder,
  pass: Option<GPUComputePass<'static>>,
  pub force_indirect_dispatch: bool,
}

impl<'a> Drop for DeviceParallelComputeCtx<'a> {
  fn drop(&mut self) {
    // make sure pass is dropped before encoder.
    self.flush_pass();
  }
}

impl<'a> DeviceParallelComputeCtx<'a> {
  pub fn new(gpu: &'a GPU) -> Self {
    let encoder = gpu.create_encoder();
    Self {
      gpu,
      encoder,
      pass: None,
      force_indirect_dispatch: false,
    }
  }

  pub fn record_pass(&mut self, f: impl FnOnce(&mut GPUComputePass, &GPUDevice)) {
    let pass = self.pass.get_or_insert_with(|| {
      let pass = self.encoder.begin_compute_pass();
      unsafe { std::mem::transmute(pass) }
    });
    f(pass, &self.gpu.device);
  }

  pub fn flush_pass(&mut self) {
    self.pass = None;
  }

  pub fn submit_recorded_work_and_continue(&mut self) {
    self.flush_pass();
    let new_encoder = self.gpu.create_encoder();
    let current_encoder = std::mem::replace(&mut self.encoder, new_encoder);

    self.gpu.submit_encoder(current_encoder);
  }

  pub fn get_or_create_compute_pipeline(
    &mut self,
    source: &(impl ShaderHashProvider + ?Sized),
    creator: impl FnOnce(&mut ComputeCx),
  ) -> GPUComputePipeline {
    let mut hasher = PipelineHasher::default();
    source.hash_pipeline_with_type_info(&mut hasher);

    self
      .gpu
      .device
      .get_or_cache_create_compute_pipeline(hasher, |builder| {
        builder.entry(|cx| {
          creator(cx);
        })
      })
  }
}

impl<T: 'static> ShaderHashProvider for Box<dyn DeviceInvocationComponent<T>> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }

  shader_hash_type_id! {}
}
