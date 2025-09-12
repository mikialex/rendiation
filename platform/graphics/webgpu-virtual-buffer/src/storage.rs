use crate::*;

#[derive(Clone)]
pub struct CombinedStorageBufferAllocator {
  internal: Arc<RwLock<CombinedBufferAllocatorInternal>>,
  for_atomic: bool,
}

impl AbstractStorageAllocator for CombinedStorageBufferAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    _device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
    _label: Option<&str>,
  ) -> BoxedAbstractBuffer {
    if !readonly && self.is_readonly() {
      panic!("readonly allocator can not allocate writeable buffer");
    }

    Box::new(self.allocate_dyn(byte_size, ty_desc))
  }

  fn get_layout(&self) -> StructLayoutTarget {
    self.internal.read().layout
  }

  fn is_readonly(&self) -> bool {
    self.internal.read().readonly
  }
}

fn rule_out_atomic_types(ty: &MaybeUnsizedValueType) {
  fn rule_out_for_single(single: &ShaderSizedValueType) {
    if let ShaderSizedValueType::Atomic(_) = single {
      panic!("atomic is not able to store into storage buffer allocator");
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
  pub fn new(gpu: &GPU, label: impl Into<String>, use_packed_layout: bool, readonly: bool) -> Self {
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
        readonly,
      ))),
      for_atomic: false,
    }
  }

  /// label must unique across binding
  pub fn new_atomic<T: AtomicityShaderNodeType>(gpu: &GPU, label: impl Into<String>) -> Self {
    Self {
      internal: Arc::new(RwLock::new(CombinedBufferAllocatorInternal::new(
        gpu,
        label,
        BufferUsages::STORAGE,
        StructLayoutTarget::Packed,
        Some(T::ATOM),
        false,
      ))),
      for_atomic: true,
    }
  }

  pub fn allocate_dyn(
    &self,
    byte_size: u64,
    ty_desc: MaybeUnsizedValueType,
  ) -> SubCombinedStorageBufferDynTyped {
    if !self.for_atomic {
      rule_out_atomic_types(&ty_desc);
    } else {
      // todo, check ty is pure atomic
    }

    assert!(byte_size % 4 == 0);
    let sub_buffer_u32_size = byte_size / 4;
    let buffer_index = self.internal.write().allocate(sub_buffer_u32_size as u32);

    SubCombinedStorageBufferDynTyped {
      buffer_index,
      ty: ty_desc,
      internal: self.internal.clone(),
    }
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
  pub fn resize(&self, new_u32_size: u32) {
    self
      .internal
      .write()
      .resize(self.buffer_index, new_u32_size);
  }
}
impl AbstractBuffer for SubCombinedStorageBufferDynTyped {
  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    let mut internal = self.internal.write();
    internal.get_sub_gpu_buffer_view(self.buffer_index).into()
  }

  fn write(&self, content: &[u8], offset: u64, _queue: &GPUQueue) {
    self
      .internal
      .write()
      .write_content(self.buffer_index, content, offset);
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    self
      .internal
      .write()
      .bind_shader_impl(bind_builder, self.ty.clone())
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    let mut internal = self.internal.write();
    internal.bind_pass(bind_builder, self.buffer_index);
  }

  fn byte_size(&self) -> u64 {
    self.get_gpu_buffer_view().unwrap().view_byte_size().into()
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    let source = self.get_gpu_buffer_view().unwrap(); // this won't fail
    let target = target.get_gpu_buffer_view().unwrap(); // this may fail
    encoder.copy_buffer_to_buffer(
      source.resource.gpu(),
      self_offset + source.desc.offset,
      target.resource.gpu(),
      target_offset + target.desc.offset,
      count,
    );
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn ref_clone(&self) -> Box<dyn AbstractBuffer> {
    Box::new(self.clone())
  }

  fn resize_gpu(
    &mut self,
    _encoder: &mut GPUCommandEncoder,
    _device: &GPUDevice,
    new_byte_size: u64,
  ) {
    assert!(new_byte_size % 4 == 0);
    self.resize(new_byte_size as u32 / 4);
  }
}
