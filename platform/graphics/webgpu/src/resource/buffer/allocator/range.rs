use crate::*;

type AllocationHandel = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GPURangeAllocateBuffer {
  ranges: FastHashMap<u32, (Range<u32>, AllocationHandel)>,
  // todo, try other allocator that support relocate and shrink??
  //
  // In the rust ecosystem, there are many allocator implementations but it's rare to find one for
  // our use case, because what we want is an allocator to manage the external memory not the
  // internal, which means the allocate does not own the memory and is unable to store internal
  // allocation states and data structures into the requested but not allocated memory space.
  allocator: xalloc::SysTlsf<u32>,
  buffer: GPUBufferResourceView,
  usage: BufferUsages,
  max_size: usize,
  /// item_byte_size must be multiple of 4 as the u32 is the minimal type on device.
  item_byte_size: usize,
}

pub struct RelocationMessage {
  pub allocation_handle: u32,
  pub new_offset: u32,
}

impl GPURangeAllocateBuffer {
  pub fn buffer(&self) -> &GPUBufferResourceView {
    &self.buffer
  }

  pub fn init_with_initial_item_count(
    device: &GPUDevice,
    init_size: usize,
    max_size: usize,
    item_byte_size: usize,
    mut usage: BufferUsages,
  ) -> Self {
    assert!(max_size >= init_size);
    assert!(item_byte_size % 4 == 0);

    // make sure we can grow buffer
    usage.insert(BufferUsages::COPY_DST | BufferUsages::COPY_SRC);

    let inner = xalloc::SysTlsf::new(init_size as u32);

    let buffer = create_gpu_buffer_zeroed((init_size * item_byte_size) as u64, usage, device);
    let buffer = buffer.create_view(Default::default());

    GPURangeAllocateBuffer {
      ranges: Default::default(),
      allocator: inner,
      buffer,
      max_size,
      item_byte_size,
      usage,
    }
  }

  /// Return Option<offset>, None means memory is full
  ///
  /// allocation_handle is a user defined u32 token, it should be unique for each allocation
  /// but could be reused. this handle is used to identify the allocation in the relocation handler
  ///
  /// when do allocating, buffer may be resized and previous allocation may be relocated, the
  /// user passed in relocation handler will be called if this happens.
  pub fn allocate(
    &mut self,
    allocation_handle: u32,
    content: &[u8],
    device: &GPUDevice,
    queue: &GPUQueue,
    relocation_handler: &mut impl FnMut(RelocationMessage),
  ) -> Option<u32> {
    let current_size: u64 = self.buffer.resource.desc.size.into();
    let current_size = current_size / self.item_byte_size as u64;
    assert!(!content.is_empty());
    assert!(content.len() % self.item_byte_size == 0);
    let required_size = (content.len() / self.item_byte_size) as u32;
    loop {
      if let Some((token, offset)) = self.allocator.alloc(required_size) {
        queue.write_buffer(
          self.buffer.resource.gpu(),
          (offset as usize * self.item_byte_size) as u64,
          bytemuck::cast_slice(content),
        );

        let previous = self
          .ranges
          .insert(allocation_handle, (offset..offset + required_size, token));
        assert!(
          previous.is_none(),
          "duplicate active allocation handle used"
        );

        break Some(offset);
      } else if self.max_size as u64 <= current_size {
        break None;
      } else {
        let grow_planed = ((current_size as f32) * 1.5) as u32;
        let real_grow_size = grow_planed
          .max(required_size + current_size as u32)
          .min(self.max_size as u32);
        self.grow(
          real_grow_size - current_size as u32,
          device,
          queue,
          relocation_handler,
        )
      }
    }
  }

  pub fn deallocate(&mut self, token: u32) {
    let (_, token) = self.ranges.remove(&token).unwrap();
    self.allocator.dealloc(token).unwrap();
  }

  fn grow(
    &mut self,
    grow_size: u32,
    device: &GPUDevice,
    queue: &GPUQueue,
    relocation_handler: &mut impl FnMut(RelocationMessage),
  ) {
    let current_size: u64 = self.buffer.resource.desc.size.into();
    let current_size = current_size / self.item_byte_size as u64;
    let new_size = current_size + grow_size as u64;

    let new_buffer =
      create_gpu_buffer_zeroed(new_size * self.item_byte_size as u64, self.usage, device);
    let new_buffer = new_buffer.create_view(Default::default());

    let mut encoder = device.create_encoder();
    let mut new_allocator = xalloc::SysTlsf::new(new_size as u32);

    // move all old data to new allocation
    self
      .ranges
      .iter_mut()
      .for_each(|(allocation_handle, (current, token))| {
        let size = current.end - current.start;
        let (new_token, new_offset) = new_allocator
          .alloc(size)
          .expect("relocation should success");

        encoder.copy_buffer_to_buffer(
          self.buffer.resource.gpu(),
          current.start as u64 * self.item_byte_size as u64,
          new_buffer.resource.gpu(),
          new_offset as u64 * self.item_byte_size as u64,
          size as u64 * self.item_byte_size as u64,
        );

        *token = new_token;
        *current = new_offset..new_offset + size;
        relocation_handler(RelocationMessage {
          allocation_handle: *allocation_handle,
          new_offset,
        })
      });

    queue.submit_encoder(encoder);

    self.buffer = new_buffer;
    self.allocator = new_allocator;
  }
}

pub struct StorageBufferRangeAllocatePool<T> {
  pool: GPURangeAllocateBuffer,
  ty: PhantomData<T>,
}

impl<T> Deref for StorageBufferRangeAllocatePool<T> {
  type Target = GPURangeAllocateBuffer;

  fn deref(&self) -> &Self::Target {
    &self.pool
  }
}
impl<T> DerefMut for StorageBufferRangeAllocatePool<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pool
  }
}

impl<T> StorageBufferRangeAllocatePool<T> {
  pub fn new(device: &GPUDevice, init_size: usize, max_size: usize) -> Self {
    Self {
      pool: GPURangeAllocateBuffer::init_with_initial_item_count(
        device,
        init_size,
        max_size,
        std::mem::size_of::<T>(),
        BufferUsages::STORAGE,
      ),
      ty: Default::default(),
    }
  }
}

impl<T> CacheAbleBindingSource for StorageBufferRangeAllocatePool<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.pool.buffer().get_binding_build_source()
  }
}

impl<T: ShaderSizedValueNodeType> ShaderBindingProvider for StorageBufferRangeAllocatePool<T> {
  type Node = ShaderReadOnlyStoragePtr<[T]>;
}
