use crate::*;

pub trait AbstractStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
  ) -> BoxedAbstractBufferDynTyped;
  fn get_layout(&self) -> StructLayoutTarget;
  fn is_readonly(&self) -> bool;
}
impl AbstractStorageAllocator for Box<dyn AbstractStorageAllocator> {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
  ) -> BoxedAbstractBufferDynTyped {
    (**self).allocate_dyn_ty(byte_size, device, ty_desc, readonly)
  }

  fn get_layout(&self) -> StructLayoutTarget {
    (**self).get_layout()
  }

  fn is_readonly(&self) -> bool {
    (**self).is_readonly()
  }
}
impl AbstractStorageAllocator for &'_ dyn AbstractStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
  ) -> BoxedAbstractBufferDynTyped {
    (**self).allocate_dyn_ty(byte_size, device, ty_desc, readonly)
  }

  fn get_layout(&self) -> StructLayoutTarget {
    (**self).get_layout()
  }

  fn is_readonly(&self) -> bool {
    (**self).is_readonly()
  }
}

pub trait AbstractStorageAllocatorExt {
  /// only valid if the allocator is config not readonly
  fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> AbstractStorageBuffer<T>;

  /// only valid if the allocator is config readonly
  fn allocate_readonly<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> AbstractReadonlyStorageBuffer<T>;
}
impl<X> AbstractStorageAllocatorExt for X
where
  X: AbstractStorageAllocator,
{
  fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> AbstractStorageBuffer<T> {
    AbstractStorageBuffer {
      buffer: self.allocate_dyn_ty(byte_size, device, T::maybe_unsized_ty(), false),
      phantom: Default::default(),
    }
  }

  fn allocate_readonly<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> AbstractReadonlyStorageBuffer<T> {
    AbstractReadonlyStorageBuffer {
      buffer: self.allocate_dyn_ty(byte_size, device, T::maybe_unsized_ty(), true),
      phantom: Default::default(),
    }
  }
}

pub struct DefaultStorageAllocator;
impl AbstractStorageAllocator for DefaultStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    _readonly: bool,
  ) -> BoxedAbstractBufferDynTyped {
    // this ty mark and read_write mark is useless actually
    let buffer = create_gpu_read_write_storage::<[u32]>(
      StorageBufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap()),
      &device,
    )
    .gpu;
    let buffer = DynTypedStorageBuffer {
      buffer,
      ty: ty_desc,
    };

    Box::new(buffer)
  }
  fn get_layout(&self) -> StructLayoutTarget {
    StructLayoutTarget::Std430
  }

  fn is_readonly(&self) -> bool {
    false
  }
}

pub type BoxedAbstractBufferDynTyped = Box<dyn AbstractBufferDynTyped>;
pub trait AbstractBufferDynTyped: DynClone {
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView;
  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue);
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> BoxedShaderPtr;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(AbstractBufferDynTyped);
impl AbstractBufferDynTyped for BoxedAbstractBufferDynTyped {
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    (**self).get_gpu_buffer_view()
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    (**self).write(content, offset, queue)
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> BoxedShaderPtr {
    (**self).bind_shader(bind_builder, registry)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    (**self).bind_pass(bind_builder)
  }
}

#[derive(Clone)]
pub struct DynTypedStorageBuffer {
  pub buffer: GPUBufferResourceView,
  pub ty: MaybeUnsizedValueType,
}
impl AbstractBufferDynTyped for DynTypedStorageBuffer {
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    self.buffer.clone()
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    queue.write_buffer(self.buffer.resource.gpu(), offset, content);
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> BoxedShaderPtr {
    let ty = self.ty.clone().into_shader_single_ty();
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ty),
      writeable_if_storage: true,
    };
    let node = bind_builder.binding_dyn(desc).using();
    Box::new(node)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind_dyn(self.buffer.get_binding_build_source());
  }
}

pub trait ComputeShaderBuilderAbstractBufferExt {
  fn bind_abstract_storage_dyn_typed(
    &mut self,
    buffer: &dyn AbstractBufferDynTyped,
  ) -> BoxedShaderPtr;
  fn bind_abstract_storage<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    buffer: &AbstractStorageBuffer<T>,
  ) -> ShaderPtrOf<T>;
  fn bind_abstract_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    buffer: &AbstractUniformBuffer<T>,
  ) -> ShaderReadonlyPtrOf<T>;
}
impl ComputeShaderBuilderAbstractBufferExt for ShaderComputePipelineBuilder {
  fn bind_abstract_storage_dyn_typed(
    &mut self,
    buffer: &dyn AbstractBufferDynTyped,
  ) -> BoxedShaderPtr {
    buffer.bind_shader(&mut self.bindgroups, &mut self.registry)
  }
  fn bind_abstract_storage<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    buffer: &AbstractStorageBuffer<T>,
  ) -> ShaderPtrOf<T> {
    let ptr = buffer
      .buffer
      .bind_shader(&mut self.bindgroups, &mut self.registry);
    T::create_view_from_raw_ptr(ptr)
  }

  fn bind_abstract_uniform<T>(
    &mut self,
    buffer: &AbstractUniformBuffer<T>,
  ) -> ShaderReadonlyPtrOf<T>
  where
    T: ShaderSizedValueNodeType + Std140,
  {
    let ptr = buffer
      .buffer
      .bind_shader(&mut self.bindgroups, &mut self.registry);
    T::create_readonly_view_from_raw_ptr(ptr)
  }
}
pub trait BindBuilderAbstractBufferExt: Sized {
  fn bind_abstract_storage_dyn_typed(&mut self, buffer: &dyn AbstractBufferDynTyped) -> &mut Self;
  fn bind_abstract_storage<T>(&mut self, buffer: &AbstractStorageBuffer<T>) -> &mut Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized;
  fn with_bind_abstract_storage<T>(mut self, buffer: &AbstractStorageBuffer<T>) -> Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
  {
    self.bind_abstract_storage(buffer);
    self
  }
  fn bind_abstract_uniform<T>(&mut self, buffer: &AbstractUniformBuffer<T>) -> &mut Self
  where
    T: ShaderSizedValueNodeType + Std140;
  fn with_bind_abstract_uniform<T>(mut self, buffer: &AbstractUniformBuffer<T>) -> Self
  where
    T: ShaderSizedValueNodeType + Std140,
  {
    self.bind_abstract_uniform(buffer);
    self
  }
}
impl BindBuilderAbstractBufferExt for BindingBuilder {
  fn bind_abstract_storage_dyn_typed(&mut self, buffer: &dyn AbstractBufferDynTyped) -> &mut Self {
    buffer.bind_pass(self);
    self
  }
  fn bind_abstract_storage<T>(&mut self, buffer: &AbstractStorageBuffer<T>) -> &mut Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
  {
    buffer.buffer.bind_pass(self);
    self
  }

  fn bind_abstract_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    buffer: &AbstractUniformBuffer<T>,
  ) -> &mut Self {
    buffer.buffer.bind_pass(self);
    self
  }
}

pub struct AbstractStorageBuffer<T: ?Sized> {
  phantom: PhantomData<T>,
  buffer: BoxedAbstractBufferDynTyped,
}

impl<T: ?Sized> AbstractStorageBuffer<T> {
  pub fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    self.buffer.get_gpu_buffer_view()
  }
}

impl<T: ?Sized> Clone for AbstractStorageBuffer<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      buffer: self.buffer.clone(),
    }
  }
}

pub struct AbstractReadonlyStorageBuffer<T: ?Sized> {
  phantom: PhantomData<T>,
  buffer: BoxedAbstractBufferDynTyped,
}

impl<T: ?Sized> Clone for AbstractReadonlyStorageBuffer<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      buffer: self.buffer.clone(),
    }
  }
}

pub struct AbstractUniformBuffer<T> {
  phantom: PhantomData<T>,
  buffer: BoxedAbstractBufferDynTyped,
}

impl<T> Clone for AbstractUniformBuffer<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      buffer: self.buffer.clone(),
    }
  }
}
