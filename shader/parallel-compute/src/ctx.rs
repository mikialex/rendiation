use crate::*;

pub struct DeviceParallelComputeCtx<'a> {
  pub gpu: GPU,
  pub encoder: &'a mut GPUCommandEncoder,
  pub pass: Option<GPUComputePass>,
  pub force_indirect_dispatch: bool,
  pub memory: &'a mut FunctionMemory,
}

unsafe impl HooksCxLike for DeviceParallelComputeCtx<'_> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    self.memory
  }

  fn memory_ref(&self) -> &FunctionMemory {
    self.memory
  }

  fn flush(&mut self) {
    let drop_cx = &mut () as *mut ();
    self.memory.flush(drop_cx);
  }

  fn is_dynamic_stage(&self) -> bool {
    true
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    // this is safe because user can not access previous retrieved state through returned self.
    let s = unsafe { std::mem::transmute_copy(&self) };

    let state = self
      .memory
      .expect_state_init(f, |_state: &mut T, _: &mut ()| {});

    (s, state)
  }
}

impl Drop for DeviceParallelComputeCtx<'_> {
  fn drop(&mut self) {
    // make sure pass is dropped
    // note, we not submit encoder here to reduce submit count
    self.flush_pass();
  }
}

impl<'a> DeviceParallelComputeCtx<'a> {
  /// note, the passed in encoder will not be submit after this ctx drop.
  pub fn new(
    gpu: &GPU,
    encoder: &'a mut GPUCommandEncoder,
    memory: &'a mut FunctionMemory,
  ) -> Self {
    Self {
      gpu: gpu.clone(),
      encoder,
      pass: None,
      force_indirect_dispatch: false,
      memory,
    }
  }

  pub fn use_rw_storage_buffer<T: Std430>(
    &mut self,
    size_require: usize,
  ) -> StorageBufferDataView<[T]> {
    let gpu = self.gpu.clone();
    let (_, output) = self.use_plain_state(|| {
      let init = ZeroedArrayByArrayLength(size_require);
      create_gpu_read_write_storage::<[T]>(init, &gpu)
    });

    if (output.item_count() as usize) < size_require {
      let init = ZeroedArrayByArrayLength(size_require);
      *output = create_gpu_read_write_storage::<[T]>(init, &gpu)
    }

    output.clone()
  }

  pub fn read_buffer(&mut self, buffer: &GPUBufferResourceView) -> ReadBufferFromStagingBuffer {
    self.encoder.read_buffer(&self.gpu.device, buffer)
  }

  pub fn read_buffer_bytes(
    &mut self,
    buffer: &GPUBufferResourceView,
  ) -> impl Future<Output = Result<Vec<u8>, rendiation_webgpu::BufferAsyncError>> {
    self.encoder.read_buffer_bytes(&self.gpu.device, buffer)
  }

  pub fn read_storage_array<T: Std430>(
    &mut self,
    buffer: &StorageBufferDataView<[T]>,
  ) -> impl Future<Output = Result<Vec<T>, rendiation_webgpu::BufferAsyncError>> {
    self
      .encoder
      .read_storage_array::<T>(&self.gpu.device, buffer)
  }
  pub fn read_sized_storage_array<T: Std430>(
    &mut self,
    buffer: &StorageBufferDataView<T>,
  ) -> impl Future<Output = Result<T, rendiation_webgpu::BufferAsyncError>> {
    self
      .encoder
      .read_sized_storage_buffer::<T>(&self.gpu.device, buffer)
  }

  pub fn record_pass<R>(&mut self, f: impl FnOnce(&mut GPUComputePass, &GPUDevice) -> R) -> R {
    let pass = self
      .pass
      .get_or_insert_with(|| self.encoder.begin_compute_pass());
    f(pass, &self.gpu.device)
  }

  pub fn flush_pass(&mut self) {
    self.pass = None;
  }

  pub fn submit_recorded_work_and_continue(&mut self) {
    self.flush_pass();
    let new_encoder = self.gpu.create_encoder();
    let current_encoder = std::mem::replace(self.encoder, new_encoder);

    self.gpu.submit_encoder(current_encoder);
  }

  pub fn get_or_create_compute_pipeline(
    &mut self,
    source: &(impl ShaderHashProvider + ?Sized),
    creator: impl FnOnce(&mut ShaderComputePipelineBuilder),
  ) -> GPUComputePipeline {
    let mut hasher = PipelineHasher::default();
    source.hash_pipeline_with_type_info(&mut hasher);

    self
      .gpu
      .device
      .get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        creator(&mut builder);
        builder
      })
  }
}

pub trait FrameCtxParallelComputeExt {
  fn access_parallel_compute<R>(&mut self, f: impl FnOnce(&mut DeviceParallelComputeCtx) -> R)
    -> R;
}

impl FrameCtxParallelComputeExt for FrameCtx<'_> {
  fn access_parallel_compute<R>(
    &mut self,
    f: impl FnOnce(&mut DeviceParallelComputeCtx) -> R,
  ) -> R {
    let mut ctx = DeviceParallelComputeCtx::new(self.gpu, &mut self.encoder, self.memory);
    let r = f(&mut ctx);
    ctx.flush_pass();
    r
  }
}

// for testing only
#[allow(unused_macros)]
#[macro_export]
macro_rules! gpu_cx {
  ($name: tt) => {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    let mut encoder = gpu.create_encoder();
    let mut memory = Default::default();
    let mut $name = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);
    let $name = &mut $name;
  };
}
