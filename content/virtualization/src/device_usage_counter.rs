use crate::*;

pub struct DeviceUsageCounter {
  counter: StorageBufferDataView<[DeviceAtomic<u32>]>,
}

impl DeviceUsageCounter {
  pub fn new(device: &GPUDevice, size: usize) -> Self {
    let init = ZeroedArrayByArrayLength(size);
    let counter = create_gpu_read_write_storage(init, device);
    Self { counter }
  }

  pub fn read_back(
    &self,
    frame_ctx: &mut FrameCtx,
  ) -> impl Future<Output = Result<Vec<u32>, BufferAsyncError>> {
    frame_ctx
      .encoder
      .read_atomic_storage_array(&frame_ctx.gpu.device, &self.counter)
  }

  pub fn build(&self, cx: &mut ShaderBindGroupBuilder) -> DeviceUsageCounterInvocation {
    DeviceUsageCounterInvocation {
      counter: cx.bind_by(&self.counter),
    }
  }

  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.counter);
  }
}

pub struct DeviceUsageCounterInvocation {
  counter: ShaderPtrOf<[DeviceAtomic<u32>]>,
}

impl DeviceUsageCounterInvocation {
  pub fn record_usage(&self, id: Node<u32>) {
    self.counter.index(id).atomic_add(val(1));
  }
}
