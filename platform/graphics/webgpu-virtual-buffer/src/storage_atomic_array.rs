use crate::*;

#[derive(Clone)]
pub struct CombinedAtomicArrayStorageBufferAllocator<T> {
  atomic_ty: PhantomData<T>,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl<T: AtomicityShaderNodeType> CombinedAtomicArrayStorageBufferAllocator<T> {
  /// label must unique across binding
  ///
  /// using compact_layout could reduce memory usage but unable to share the data with host or other shader easily
  pub fn new(gpu: &GPU, label: impl Into<String>) -> Self {
    Self {
      atomic_ty: PhantomData,
      internal: Arc::new(RwLock::new(CombinedBufferAllocatorInternal::new(
        gpu,
        label,
        BufferUsages::STORAGE,
        StructLayoutTarget::Packed,
        Some(T::ATOM),
      ))),
    }
  }
  pub fn allocate_atomic_array(&self, atomic_count: u32) -> SubCombinedAtomicArrayStorageBuffer<T> {
    let buffer_index = self.internal.write().allocate(atomic_count);

    SubCombinedAtomicArrayStorageBuffer {
      buffer_index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }

  pub fn allocate_single_atomic(&self) -> SubCombinedSingleAtomicStorageBuffer<T> {
    let buffer_index = self.internal.write().allocate(1);

    SubCombinedSingleAtomicStorageBuffer {
      buffer_index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }
}

#[derive(Clone)]
pub struct SubCombinedAtomicArrayStorageBuffer<T> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: usize,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl<T: AtomicityShaderNodeType> SubCombinedAtomicArrayStorageBuffer<T> {
  /// resize the sub buffer to new size, the content will be preserved moved to new place
  ///
  /// once resize, the merged buffer must rebuild;
  pub fn resize(&self, new_u32_size: u32) {
    self
      .internal
      .write()
      .resize(self.buffer_index, new_u32_size);
  }

  pub fn write_content(&self, content: &[u32], offset: u64) {
    let content_in_bytes = cast_slice(content);
    self
      .internal
      .write()
      .write_content(self.buffer_index, content_in_bytes, offset);
  }
}

impl<T> AbstractStorageBuffer<[DeviceAtomic<T>]> for SubCombinedAtomicArrayStorageBuffer<T>
where
  T: AtomicityShaderNodeType,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    let mut internal = self.internal.write();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn write(&self, content: &[u8], offset: u64, _queue: &GPUQueue) {
    self.write_content(cast_slice(content), offset);
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> ShaderPtrOf<[DeviceAtomic<T>]> {
    self
      .internal
      .write()
      .bind_shader_storage::<[DeviceAtomic<T>]>(bind_builder, reg)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.bind_pass(bind_builder, self.buffer_index);
  }
}

#[derive(Clone)]
pub struct SubCombinedSingleAtomicStorageBuffer<T> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: usize,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl<T> AbstractStorageBuffer<DeviceAtomic<T>> for SubCombinedSingleAtomicStorageBuffer<T>
where
  T: AtomicityShaderNodeType,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    let mut internal = self.internal.write();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn write(&self, content: &[u8], offset: u64, _queue: &GPUQueue) {
    self
      .internal
      .write()
      .write_content(self.buffer_index, content, offset);
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> ShaderPtrOf<DeviceAtomic<T>> {
    self
      .internal
      .write()
      .bind_shader_storage::<DeviceAtomic<T>>(bind_builder, reg)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.bind_pass(bind_builder, self.buffer_index);
  }
}
