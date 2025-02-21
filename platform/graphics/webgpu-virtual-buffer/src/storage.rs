use crate::*;

#[derive(Clone)]
pub struct CombinedStorageBufferAllocator {
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl CombinedStorageBufferAllocator {
  /// label must unique across binding
  ///
  /// using compact_layout could reduce memory usage but unable to share the data with host or other shader easily
  pub fn new(label: impl Into<String>, use_packed_layout: bool) -> Self {
    Self {
      internal: Arc::new(RwLock::new(CombinedBufferAllocatorInternal::new(
        label,
        BufferUsages::STORAGE,
        if use_packed_layout {
          StructLayoutTarget::Packed
        } else {
          StructLayoutTarget::Std430
        },
      ))),
    }
  }
  pub fn allocate<T: Std430MaybeUnsized>(
    &mut self,
    sub_buffer_u32_size: u32,
  ) -> SubCombinedStorageBuffer<T> {
    let buffer_index = self.internal.write().allocate(sub_buffer_u32_size);

    SubCombinedStorageBuffer {
      buffer_index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }

  pub fn rebuild(&self, gpu: &GPU) {
    self.internal.write().rebuild(gpu);
  }
}

pub struct SubCombinedStorageBuffer<T: ?Sized> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: usize,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
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

impl<T: ShaderMaybeUnsizedValueNodeType + ?Sized> SubCombinedStorageBuffer<T> {
  /// resize the sub buffer to new size, the content will be moved
  ///
  /// once resize, the merged buffer must rebuild;
  pub fn resize(&mut self, new_u32_size: u32) {
    self
      .internal
      .write()
      .resize(self.buffer_index, new_u32_size);
  }

  pub fn write_content(&mut self, content: &[u8], queue: &GPUQueue) {
    self
      .internal
      .write()
      .write_content(self.buffer_index, content, queue);
  }

  pub fn bind_shader_impl(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    self
      .internal
      .read()
      .bind_shader_storage::<T>(bind_builder, registry, self.buffer_index)
  }
}

impl<T> AbstractStorageBuffer<T> for SubCombinedStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferView {
    let internal = self.internal.read();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    self.bind_shader_impl(bind_builder, reg)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let internal = self.internal.read();
    internal.bind_pass(bind_builder);
  }
}
