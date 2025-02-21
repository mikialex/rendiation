use crate::*;

pub struct CombinedUniformBufferAllocator {
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl CombinedUniformBufferAllocator {
  /// label must unique across binding
  pub fn new(label: impl Into<String>) -> Self {
    Self {
      internal: Arc::new(RwLock::new(CombinedBufferAllocatorInternal::new(
        label,
        BufferUsages::UNIFORM,
      ))),
    }
  }
  pub fn allocate<T: Std140 + ShaderSizedValueNodeType>(
    &mut self,
    sub_buffer_u32_size: u32,
  ) -> SubCombinedUniformBuffer<T> {
    let buffer_index = self.internal.write().allocate(sub_buffer_u32_size);

    SubCombinedUniformBuffer {
      buffer_index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }

  pub fn rebuild(&self, gpu: &GPU) {
    self.internal.write().rebuild(gpu);
  }
}

pub struct SubCombinedUniformBuffer<T> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: usize,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl<T> Clone for SubCombinedUniformBuffer<T> {
  fn clone(&self) -> Self {
    Self {
      buffer_index: self.buffer_index,
      phantom: self.phantom,
      internal: self.internal.clone(),
    }
  }
}

impl<T: Std140 + ShaderSizedValueNodeType> SubCombinedUniformBuffer<T> {
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
  ) -> ShaderReadonlyPtrOf<T> {
    self
      .internal
      .read()
      .bind_shader_uniform::<T>(bind_builder, registry, self.buffer_index)
  }
}

impl<T> AbstractUniformBuffer<T> for SubCombinedUniformBuffer<T>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferView {
    let internal = self.internal.read();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T> {
    self.bind_shader_impl(bind_builder, reg)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let internal = self.internal.read();
    internal.bind_pass(bind_builder);
  }
}
