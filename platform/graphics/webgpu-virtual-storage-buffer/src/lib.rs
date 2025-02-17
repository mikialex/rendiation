use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

/// this feature allows user create rw storage buffer from a single buffer pool
/// to workaround the binding limitation on some platform.
#[derive(Default)]
pub struct StorageBufferMergeAllocator {
  internal: Arc<RwLock<StorageBufferMergeAllocatorInternal>>,
}

struct StorageBufferMergeAllocatorInternal {
  binding_epoch: u32,
  buffer: Option<StorageBufferDataView<[u32]>>,
  buffer_need_rebuild: bool,
  sub_buffer_allocation_u32_offset: Vec<u32>,
  sub_buffer_u32_size_requirements: Vec<u32>,
}

impl Default for StorageBufferMergeAllocatorInternal {
  fn default() -> Self {
    Self {
      binding_epoch: 0,
      buffer: None,
      buffer_need_rebuild: true,
      sub_buffer_allocation_u32_offset: Default::default(),
      sub_buffer_u32_size_requirements: Default::default(),
    }
  }
}

impl StorageBufferMergeAllocator {
  pub fn allocate<T: Std430MaybeUnsized>(
    &mut self,
    sub_buffer_u32_size: u32,
  ) -> SubMergedStorageBuffer<T> {
    let mut internal = self.internal.write();
    internal.buffer_need_rebuild = true;
    let index = internal.sub_buffer_u32_size_requirements.len() as u32;
    internal
      .sub_buffer_u32_size_requirements
      .push(sub_buffer_u32_size);

    SubMergedStorageBuffer {
      buffer_index: index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }

  pub fn rebuild(&mut self, gpu: &GPU) {
    // let full_size_requirement: u32 = self.sub_buffer_info.iter().sum();
    // let new_shared_buffer = todo!();
    let internal = self.internal.write();
    // todo data movement
    //
  }
}

pub struct SubMergedStorageBuffer<T: ?Sized> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: u32,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<StorageBufferMergeAllocatorInternal>>,
}

impl<T: ?Sized> SubMergedStorageBuffer<T> {
  /// resize the sub buffer to new size, the content will be moved
  ///
  /// once resize, the merged buffer must rebuild;
  pub fn resize(&mut self, new_u32_size: u32) {
    let mut internal = self.internal.write();
    internal.sub_buffer_u32_size_requirements[self.buffer_index as usize] = new_u32_size;
    internal.buffer_need_rebuild = true;
  }

  pub fn write_content(&mut self, content: &[u8], queue: &GPUQueue) {
    let buffer = self.expect_buffer();
    let offset = self.internal.read().sub_buffer_allocation_u32_offset[self.buffer_index as usize];
    let offset = (offset * 4) as u64;
    queue.write_buffer(buffer.buffer.gpu(), offset, content);
  }

  pub fn expect_buffer(&self) -> StorageBufferDataView<[u32]> {
    let err = "merged buffer not yet build";
    let internal = self.internal.read();
    let buffer = internal.buffer.clone();
    assert!(!internal.buffer_need_rebuild, "{err}");
    buffer.expect(err)
  }

  pub fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
  ) -> ShaderStorageVirtualTypedPtrNode<T> {
    let buffer = self.expect_buffer();
    let array = bind_builder.bind_by(&buffer);
    let base_offset = array.index(self.buffer_index).load();
    let ptr = ShaderStorageVirtualPtrNode {
      array,
      offset: base_offset,
    };
    ShaderStorageVirtualTypedPtrNode {
      ty: PhantomData,
      ptr,
    }
  }

  pub fn bind_pass(&self, bind_builder: &mut BindGroupBuilder) {
    let mut internal = self.internal.write();
    if internal.binding_epoch == 0 {
      // bind_builder.bind(&internal.buffer.expect("merged buffer not yet build"));
    }
    internal.binding_epoch += 1;
    if internal.binding_epoch == internal.sub_buffer_u32_size_requirements.len() as u32 {
      internal.binding_epoch = 0;
    }
  }
}

pub struct ShaderStorageVirtualPtrNode {
  pub array: StorageNode<[u32]>,
  pub offset: Node<u32>,
}

pub struct ShaderStorageVirtualTypedPtrNode<T: ?Sized> {
  pub ty: PhantomData<T>,
  pub ptr: ShaderStorageVirtualPtrNode,
}

impl<T: ShaderSizedValueNodeType> ShaderStorageVirtualTypedPtrNode<T> {
  pub fn load(&self) -> Node<T> {
    Node::load_from_u32_buffer(self.ptr.array, self.ptr.offset)
  }

  pub fn store(&self, node: Node<T>) {
    node.store_into_u32_buffer(self.ptr.array, self.ptr.offset);
  }
}

// todo create a bunch of macro to convert node to node and load fn
