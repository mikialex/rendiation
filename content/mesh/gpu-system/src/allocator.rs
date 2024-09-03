use core::ops::Range;

use fast_hash_collection::*;
use rendiation_webgpu::*;

type AllocationHandel = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

pub struct GPUSubAllocateBuffer {
  ranges: FastHashMap<u32, (Range<u32>, AllocationHandel)>,
  // todo should we try other allocator that support relocate and shrink??
  //
  // In the rust ecosystem, there are many allocator implementations but it's rare to find one for
  // our use case, because what we want is an allocator to manage the external memory not the
  // internal, which means the allocate does not own the memory and is unable to store internal
  // allocation states and data structures into the requested but not allocated memory space.
  allocator: xalloc::SysTlsf<u32>,
  buffer: GPUBufferResourceView,
  usage: BufferUsages,
  max_size: usize,
  /// the reason we allocate by item instead of bytes is that allocation have to be aligned to type
  item_byte_size: usize,
}

pub struct RelocationMessage {
  pub allocation_handle: u32,
  pub new_offset: u32,
}

impl GPUSubAllocateBuffer {
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

pub struct GPUSubAllocateResult {
  pub token: u32,
  pub allocate_offset: u32,
}

impl GPUSubAllocateBuffer {
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

    // make sure we can grow buffer
    usage.insert(BufferUsages::COPY_DST | BufferUsages::COPY_SRC);

    let inner = xalloc::SysTlsf::new(init_size as u32);

    let buffer = create_gpu_buffer_zeroed((init_size * item_byte_size) as u64, usage, device);
    let buffer = buffer.create_view(Default::default());

    GPUSubAllocateBuffer {
      ranges: Default::default(),
      allocator: inner,
      buffer,
      max_size,
      item_byte_size,
      usage,
    }
  }

  /// deallocate handled by drop, return None means oom
  pub fn allocate(
    &mut self,
    allocation_handle: u32,
    content: &[u8],
    device: &GPUDevice,
    queue: &GPUQueue,
    relocation_handler: &mut impl FnMut(RelocationMessage),
  ) -> Option<GPUSubAllocateResult> {
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

        break GPUSubAllocateResult {
          token: allocation_handle,
          allocate_offset: offset,
        }
        .into();
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
}
