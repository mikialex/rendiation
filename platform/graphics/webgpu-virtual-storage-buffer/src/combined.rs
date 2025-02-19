use crate::*;

/// this feature allows user create rw storage buffer from a single buffer pool
/// to workaround the binding limitation on some platform.
pub struct CombinedStorageBufferAllocator {
  internal: Arc<RwLock<CombinedStorageBufferAllocatorInternal>>,
}

impl CombinedStorageBufferAllocator {
  /// label must unique
  pub fn new(label: impl Into<String>) -> Self {
    Self {
      internal: Arc::new(RwLock::new(CombinedStorageBufferAllocatorInternal {
        label: label.into(),
        buffer: None,
        buffer_need_rebuild: true,
        sub_buffer_allocation_u32_offset: Default::default(),
        sub_buffer_u32_size_requirements: Default::default(),
      })),
    }
  }
}

struct CombinedStorageBufferAllocatorInternal {
  label: String,
  buffer: Option<StorageBufferDataView<[u32]>>,
  buffer_need_rebuild: bool,
  sub_buffer_allocation_u32_offset: Vec<u32>,
  sub_buffer_u32_size_requirements: Vec<u32>,
}

impl CombinedStorageBufferAllocator {
  pub fn allocate<T: Std430MaybeUnsized>(
    &mut self,
    sub_buffer_u32_size: u32,
  ) -> SubCombinedStorageBuffer<T> {
    let mut internal = self.internal.write();
    internal.buffer_need_rebuild = true;
    let index = internal.sub_buffer_u32_size_requirements.len() as u32;
    internal
      .sub_buffer_u32_size_requirements
      .push(sub_buffer_u32_size);

    SubCombinedStorageBuffer {
      buffer_index: index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }

  // todo, old data movement
  pub fn rebuild(&mut self, gpu: &GPU) {
    let mut internal = self.internal.write();
    if !internal.buffer_need_rebuild && internal.buffer.is_some() {
      return;
    }

    internal.sub_buffer_allocation_u32_offset = internal
      .sub_buffer_u32_size_requirements
      .iter()
      .scan(0, |offset, size| {
        let o = *offset;
        *offset += size;
        Some(o)
      })
      .collect();

    let full_size_requirement: u32 = internal.sub_buffer_u32_size_requirements.iter().sum();

    let buffer = create_gpu_read_write_storage(full_size_requirement as usize, &gpu.device);
    internal.buffer = Some(buffer);

    internal.buffer_need_rebuild = false;
  }
}

pub struct SubCombinedStorageBuffer<T: ?Sized> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: u32,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<CombinedStorageBufferAllocatorInternal>>,
}

impl<T: ?Sized> Clone for SubCombinedStorageBuffer<T> {
  fn clone(&self) -> Self {
    Self {
      buffer_index: self.buffer_index,
      phantom: self.phantom,
      internal: self.internal.clone(),
    }
  }
}

impl<T: ?Sized> SubCombinedStorageBuffer<T> {
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
    registry: &mut SemanticRegistry,
  ) -> ShaderStorageVirtualTypedPtrNode<T> {
    let label = self.internal.read().label.clone();
    // let array = registry.dynamic_semantic.entry(label).or_insert_with(|| {
    //   let buffer = self.expect_buffer();
    //   bind_builder.bind_by(&buffer).cast_untyped_node()
    // });
    // let array: StorageNode<[u32]> = unsafe { array.cast_type() };
    let array: ShaderAccessorOf<[u32]> = todo!();

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
    let buffer = self.expect_buffer();
    bind_builder.bind_if_not_exist_before(buffer.get_binding_build_source());
  }
}

#[derive(Clone)]
pub struct ShaderStorageVirtualPtrNode {
  // todo use U32BufferLoadStoreSource
  pub array: ShaderAccessorOf<[u32]>,
  pub offset: Node<u32>,
}

pub struct ShaderStorageVirtualTypedPtrNode<T: ?Sized> {
  pub ty: PhantomData<T>,
  pub ptr: ShaderStorageVirtualPtrNode,
}

impl<T: ?Sized> Clone for ShaderStorageVirtualTypedPtrNode<T> {
  fn clone(&self) -> Self {
    Self {
      ty: self.ty,
      ptr: self.ptr.clone(),
    }
  }
}

impl<T: ShaderSizedValueNodeType> ShaderStorageVirtualTypedPtrNode<T> {
  pub fn load(&self) -> Node<T> {
    Node::load_from_u32_buffer(&self.ptr.array, self.ptr.offset)
  }

  pub fn store(&self, node: Node<T>) {
    node.store_into_u32_buffer(&self.ptr.array, self.ptr.offset);
  }
}

impl AbstractShaderPtr for ShaderStorageVirtualPtrNode {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    todo!()
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    todo!()
  }

  fn array_length(&self) -> Node<u32> {
    todo!()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    todo!()
  }

  fn store(&self, value: ShaderNodeRawHandle) {
    todo!()
  }
  fn raw_ptr(&self) -> ShaderNodeRawHandle {
    todo!()
  }
}

impl<T> AbstractStorageBuffer<T> for SubCombinedStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  type ShaderPtr = ShaderStorageVirtualPtrNode;
  fn get_gpu_buffer_view(&self) -> &GPUBufferView {
    todo!()
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderAccessorOf<T> {
    let ptr = todo!();
    T::create_accessor_from_raw_ptr(ptr)
  }
}
