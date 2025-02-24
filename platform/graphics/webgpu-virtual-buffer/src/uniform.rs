use crate::*;

#[derive(Clone)]
pub struct CombinedUniformBufferAllocator {
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl CombinedUniformBufferAllocator {
  /// label must unique across binding
  pub fn new(gpu: &GPU, label: impl Into<String>) -> Self {
    Self {
      internal: Arc::new(RwLock::new(CombinedBufferAllocatorInternal::new(
        gpu,
        label,
        BufferUsages::UNIFORM,
        StructLayoutTarget::Std140,
        None,
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

  pub fn rebuild(&self) {
    self.internal.write().rebuild();
  }
}

#[derive(Clone)]
pub struct SubCombinedUniformBuffer<T> {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: usize,
  phantom: std::marker::PhantomData<T>,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

impl<T: Std140 + ShaderSizedValueNodeType> SubCombinedUniformBuffer<T> {
  /// resize the sub buffer to new size, the content will be preserved moved to new place
  ///
  /// once resize, the merged buffer must rebuild;
  pub fn resize(&mut self, new_u32_size: u32) {
    self
      .internal
      .write()
      .resize(self.buffer_index, new_u32_size);
  }

  pub fn write_content(&mut self, content: &[u8]) {
    self
      .internal
      .write()
      .write_content(self.buffer_index, content);
  }
}

impl<T> AbstractUniformBuffer<T> for SubCombinedUniformBuffer<T>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    let internal = self.internal.read();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T> {
    self
      .internal
      .write()
      .bind_shader_uniform::<T>(bind_builder, reg)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.bind_pass(bind_builder, self.buffer_index);
  }
}
