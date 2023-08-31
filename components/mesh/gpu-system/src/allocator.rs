use core::alloc::Layout;
use core::num::NonZeroU64;
use core::num::NonZeroUsize;
use core::ptr::NonNull;
use core::{marker::PhantomData, ops::Range};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::Weak;

use bytemuck::Pod;
use rendiation_webgpu::*;

pub struct GPUSubAllocateBuffer<T> {
  inner: Arc<RwLock<GPUSubAllocateBufferInner<T>>>,
}

struct GPUSubAllocateBufferInner<T> {
  phantom: PhantomData<T>,
  // should we try other allocator that support relocate and shrink??
  inner: buddy_system_allocator::Heap<32>,
  buffer: GPUBuffer,
  usage: BufferUsages,
  max_byte_size: usize,
}

impl<T> GPUSubAllocateBufferInner<T> {
  fn grow(&mut self, extra_bytes: usize, device: &GPUDevice, queue: &GPUQueue) {
    let current_size: u64 = self.buffer.size().into();
    let new_size = current_size + extra_bytes as u64;
    let new_buffer = GPUBuffer::create(
      device,
      BufferInit::Zeroed(NonZeroU64::new(new_size).unwrap()),
      self.usage,
    );
    // should we batch these call?
    let mut encoder = device.create_encoder();
    encoder.copy_buffer_to_buffer(self.buffer.gpu(), 0, new_buffer.gpu(), 0, current_size);
    queue.submit(Some(encoder.finish()));
    self.buffer = new_buffer;
  }
}

struct GPUSubAllocateBufferToken<T> {
  pub byte_range: Range<usize>,
  alloc: Weak<RwLock<GPUSubAllocateBufferInner<T>>>,
}

impl<T> Drop for GPUSubAllocateBufferToken<T> {
  fn drop(&mut self) {
    if let Some(alloc) = self.alloc.upgrade() {
      let ptr =
        NonNull::<u8>::dangling().with_addr(NonZeroUsize::new(self.byte_range.start).unwrap());
      alloc
        .write()
        .unwrap()
        .inner
        .dealloc(ptr, Layout::new::<T>())
    }
  }
}

impl<T> GPUSubAllocateBuffer<T> {
  pub fn get_buffer(&self) -> GPUBuffer {
    todo!()
  }

  pub fn init_with_initial_item_count(
    device: &GPUDevice,
    count: usize,
    max_count: usize,
    usage: BufferUsages,
  ) -> Self {
    assert!(max_count >= count);
    let init_byte_size = std::mem::size_of::<T>() * count;
    let mut inner = buddy_system_allocator::Heap::<32>::empty();
    unsafe { buddy_system_allocator::Heap::<32>::init(&mut inner, 0, init_byte_size) };

    let buffer = GPUBuffer::create(
      device,
      BufferInit::Zeroed(NonZeroU64::new(init_byte_size as u64).unwrap()),
      usage,
    );

    let inner = GPUSubAllocateBufferInner {
      phantom: PhantomData,
      inner,
      buffer,
      max_byte_size: max_count * std::mem::size_of::<T>(),
      usage,
    };

    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  /// deallocate handled by drop, return None means oom
  pub fn allocate(
    &self,
    content: &[T],
    device: &GPUDevice,
    queue: &GPUQueue,
  ) -> Option<GPUSubAllocateBufferToken<T>>
  where
    T: Pod,
  {
    let new_byte_size = std::mem::size_of_val(content);
    let mut alloc = self.inner.write().unwrap();
    loop {
      if let Ok(ptr) = alloc.inner.alloc(Layout::new::<T>()) {
        let offset: usize = ptr.addr().into();
        queue.write_buffer(
          alloc.buffer.gpu(),
          offset as u64,
          bytemuck::cast_slice(content),
        );

        break GPUSubAllocateBufferToken {
          byte_range: offset..offset + new_byte_size,
          alloc: Arc::downgrade(&self.inner),
        }
        .into();
      } else if alloc.max_byte_size as u64 > alloc.buffer.size().into() {
        break None;
      } else {
        alloc.grow(new_byte_size, device, queue)
      }
    }
  }
}
