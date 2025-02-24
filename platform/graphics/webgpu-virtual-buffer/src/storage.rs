use crate::*;

#[derive(Clone)]
pub struct CombinedStorageBufferAllocator {
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}

fn rule_out_atomic_types(ty: &MaybeUnsizedValueType) {
  fn rule_out_for_single(single: &ShaderSizedValueType) {
    if let ShaderSizedValueType::Atomic(_) = single {
      panic!("atomic is not able to store into storage buffer allocator, use SubCombinedAtomicArrayStorageBuffer instead");
    }
  }

  match ty {
    MaybeUnsizedValueType::Sized(ty) => rule_out_for_single(ty),
    MaybeUnsizedValueType::Unsized(ty) => match ty {
      ShaderUnSizedValueType::UnsizedArray(ty) => rule_out_for_single(ty),
      ShaderUnSizedValueType::UnsizedStruct(ty) => {
        ty.sized_fields
          .iter()
          .map(|v| &v.ty)
          .for_each(rule_out_for_single);
        rule_out_for_single(&ty.last_dynamic_array_field.1)
      }
    },
  }
}

impl CombinedStorageBufferAllocator {
  /// label must unique across binding
  ///
  /// using compact_layout could reduce memory usage but unable to share the data with host or other shader easily
  pub fn new(gpu: &GPU, label: impl Into<String>, use_packed_layout: bool) -> Self {
    Self {
      internal: Arc::new(RwLock::new(CombinedBufferAllocatorInternal::new(
        gpu,
        label,
        BufferUsages::STORAGE,
        if use_packed_layout {
          StructLayoutTarget::Packed
        } else {
          StructLayoutTarget::Std430
        },
        None,
      ))),
    }
  }
  pub fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
  ) -> SubCombinedStorageBuffer<T> {
    rule_out_atomic_types(&T::maybe_unsized_ty());
    assert!(byte_size % 4 == 0);
    let sub_buffer_u32_size = byte_size / 4;
    let buffer_index = self.internal.write().allocate(sub_buffer_u32_size as u32);

    SubCombinedStorageBuffer {
      buffer_index,
      phantom: PhantomData,
      internal: self.internal.clone(),
    }
  }

  pub fn allocate_dyn(
    &self,
    byte_size: u64,
    ty_desc: MaybeUnsizedValueType,
  ) -> SubCombinedStorageBufferDynTyped {
    rule_out_atomic_types(&ty_desc);
    assert!(byte_size % 4 == 0);
    let sub_buffer_u32_size = byte_size / 4;
    let buffer_index = self.internal.write().allocate(sub_buffer_u32_size as u32);

    SubCombinedStorageBufferDynTyped {
      buffer_index,
      ty: ty_desc,
      internal: self.internal.clone(),
    }
  }

  pub fn rebuild(&self) {
    self.internal.write().rebuild();
  }
}

#[derive(Clone)]
pub struct SubCombinedStorageBufferDynTyped {
  /// user should make sure the index is stable across the binding to avoid hash this index.
  buffer_index: usize,
  ty: MaybeUnsizedValueType,
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
}
impl SubCombinedStorageBufferDynTyped {
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
impl AbstractStorageBufferDynTyped for SubCombinedStorageBufferDynTyped {
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    let internal = self.internal.read();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> BoxedShaderPtr {
    self
      .internal
      .write()
      .bind_shader_impl(bind_builder, reg, self.ty.clone())
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.bind_pass(bind_builder, self.buffer_index);
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

impl<T> AbstractStorageBuffer<T> for SubCombinedStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    let internal = self.internal.read();
    internal.get_sub_gpu_buffer_view(self.buffer_index)
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    self
      .internal
      .write()
      .bind_shader_storage::<T>(bind_builder, reg)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.bind_pass(bind_builder, self.buffer_index);
  }
}
