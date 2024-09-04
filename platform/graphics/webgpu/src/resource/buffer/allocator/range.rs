use crate::*;

type AllocationHandel = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GPURangeAllocateBuffer<T> {
  used_count: u32,
  // todo, remove this if we can get offset from handle in allocator
  ranges: FastHashMap<u32, (u32, AllocationHandel)>,
  // todo, try other allocator that support relocate and shrink??
  //
  // In the rust ecosystem, there are many allocator implementations but it's rare to find one for
  // our use case, because what we want is an allocator to manage the external memory not the
  // internal, which means the allocate does not own the memory and is unable to store internal
  // allocation states and data structures into the requested but not allocated memory space.
  allocator: xalloc::SysTlsf<u32>,
  buffer: T,
  gpu: GPU,
}

impl<T> GPURangeAllocateBuffer<T>
where
  T: ResizableLinearStorage + GPULinearStorage + LinearStorageDirectAccess,
{
  pub fn new(gpu: &GPU, buffer: T) -> Self {
    Self {
      allocator: xalloc::SysTlsf::new(buffer.max_size()),
      buffer,
      gpu: gpu.clone(),
      used_count: 0,
      ranges: Default::default(),
    }
  }

  /// return if grow success
  fn relocate(
    &mut self,
    new_size: u32,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> bool {
    // resize the underlayer buffer
    let origin_gpu_buffer = self.buffer.raw_gpu().clone();
    if !self.buffer.resize(new_size) {
      return false;
    }
    let new_gpu_buffer = self.buffer.raw_gpu().clone();

    // move the data

    let mut encoder = self.gpu.device.create_encoder();
    let mut new_allocator = xalloc::SysTlsf::new(new_size);

    // make sure any pending mutation is applied
    self.buffer.update_gpu(&mut encoder);

    let item_byte_width = std::mem::size_of::<T::Item>() as u32;
    let mut new_ranges = FastHashMap::default();
    self.ranges.iter_mut().for_each(|(offset, (size, _))| {
      let (new_token, new_offset) = new_allocator
        .alloc(*size)
        .expect("relocation should success");

      encoder.copy_buffer_to_buffer(
        origin_gpu_buffer.resource.gpu(),
        *offset as u64 * item_byte_width as u64,
        new_gpu_buffer.resource.gpu(),
        new_offset as u64 * item_byte_width as u64,
        *size as u64 * item_byte_width as u64,
      );

      new_ranges.insert(new_offset, (*size, new_token));
      relocation_handler(RelocationMessage {
        previous_offset: *offset,
        new_offset,
      })
    });

    self.gpu.queue.submit_encoder(encoder);

    self.allocator = new_allocator;
    self.ranges = new_ranges;

    true
  }
}

impl<T: LinearStorageBase> LinearStorageBase for GPURangeAllocateBuffer<T> {
  type Item = T::Item;
  fn max_size(&self) -> u32 {
    self.buffer.max_size()
  }
}

impl<T: GPULinearStorage> GPULinearStorage for GPURangeAllocateBuffer<T> {
  type GPUType = T::GPUType;

  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder) {
    self.buffer.update_gpu(encoder)
  }
  fn gpu(&self) -> &Self::GPUType {
    self.buffer.gpu()
  }
  fn raw_gpu(&self) -> &GPUBufferResourceView {
    self.buffer.raw_gpu()
  }
}

impl<T: LinearStorageBase> AllocatorStorageBase for GPURangeAllocateBuffer<T> {
  fn current_used(&self) -> u32 {
    self.used_count
  }

  fn try_compact(&mut self, _: &mut dyn FnMut(RelocationMessage)) {
    // not supported yet, but it's ok
  }

  fn try_reserve_used(&mut self, _: u32, _: &mut dyn FnMut(RelocationMessage)) {
    // not supported yet, but it's ok
  }
}

impl<T> RangeAllocatorStorage for GPURangeAllocateBuffer<T>
where
  T: ResizableLinearStorage + LinearStorageDirectAccess + GPULinearStorage,
{
  fn remove(&mut self, idx: u32) {
    let (_, token) = self.ranges.remove(&idx).unwrap();
    self.allocator.dealloc(token).unwrap();
  }

  fn set_values(
    &mut self,
    v: &[Self::Item],
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32> {
    assert!(!v.is_empty());
    let required_size = v.len() as u32;
    loop {
      if let Some((token, offset)) = self.allocator.alloc(required_size) {
        self.buffer.set_values(offset, v);
        self.ranges.insert(offset, (required_size, token));

        break Some(offset);
      } else if !self.relocate(self.buffer.max_size() + required_size, relocation_handler) {
        return None;
      }
    }
  }
}

// I want use tait but hit this: https://github.com/rust-lang/rust/issues/129954
pub type StorageBufferRangeAllocatePool<T> = GPURangeAllocateBuffer<
  CustomGrowBehaviorMaintainer<
    GPUStorageDirectQueueUpdate<ResizableGPUBuffer<StorageBufferDataView<[T]>>>,
  >,
>;

pub fn create_storage_buffer_allocate_pool<T: Std430>(
  gpu: &GPU,
  init_size: u32,
  max_size: u32,
) -> StorageBufferRangeAllocatePool<T> {
  let buffer = StorageBufferDataView::<[T]>::create_by(
    &gpu.device,
    StorageBufferInit::Zeroed(NonZeroU64::new(init_size as u64).unwrap()),
  );

  let buffer = ResizableGPUBuffer {
    gpu: buffer,
    ctx: gpu.clone(),
  }
  .with_queue_direct_update(&gpu.queue)
  .with_grow_behavior(|size| {
    //
    None
  });

  GPURangeAllocateBuffer::new(gpu, buffer)
}
