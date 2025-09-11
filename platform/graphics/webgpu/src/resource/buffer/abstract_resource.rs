use crate::*;

pub trait AbstractStorageAllocator: DynClone {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
  ) -> BoxedAbstractBuffer;
  fn get_layout(&self) -> StructLayoutTarget;
  fn is_readonly(&self) -> bool;
}
dyn_clone::clone_trait_object!(AbstractStorageAllocator);
impl AbstractStorageAllocator for Box<dyn AbstractStorageAllocator> {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
  ) -> BoxedAbstractBuffer {
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
  ) -> BoxedAbstractBuffer {
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

#[derive(Clone)]
pub struct DefaultStorageAllocator;
impl AbstractStorageAllocator for DefaultStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
  ) -> BoxedAbstractBuffer {
    // this ty mark and read_write mark is useless actually
    let buffer = create_gpu_read_write_storage::<[u32]>(
      StorageBufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap()),
      &device,
    )
    .gpu;
    let buffer = DynTypedStorageBuffer {
      buffer,
      ty: ty_desc,
      readonly,
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

pub type BoxedAbstractBuffer = Box<dyn AbstractBuffer>;
pub trait AbstractBuffer: DynClone {
  fn byte_size(&self) -> u64;
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_byte_size: u64);
  fn ref_clone(&self) -> Box<dyn AbstractBuffer>;
  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue);
  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  );
  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
  fn as_any(&self) -> &dyn Any;
  /// this is not possible(return None) if we using texture as the implementation
  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView>;
}
dyn_clone::clone_trait_object!(AbstractBuffer);
impl AbstractBuffer for BoxedAbstractBuffer {
  fn ref_clone(&self) -> Box<dyn AbstractBuffer> {
    (**self).ref_clone()
  }

  fn resize_gpu(
    &mut self,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
    new_byte_size: u64,
  ) {
    (**self).resize_gpu(encoder, device, new_byte_size)
  }
  fn byte_size(&self) -> u64 {
    (**self).byte_size()
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    (**self).write(content, offset, queue)
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    (**self).bind_shader(bind_builder)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    (**self).bind_pass(bind_builder)
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    (**self).copy_buffer_to_buffer(target, self_offset, target_offset, count, encoder)
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    (**self).get_gpu_buffer_view()
  }

  fn as_any(&self) -> &dyn Any {
    (**self).as_any()
  }
}

#[derive(Clone)]
pub struct DynTypedStorageBuffer {
  pub buffer: GPUBufferResourceView,
  pub ty: MaybeUnsizedValueType,
  pub readonly: bool,
}
impl AbstractBuffer for DynTypedStorageBuffer {
  fn ref_clone(&self) -> Box<dyn AbstractBuffer> {
    Box::new(self.clone())
  }

  fn resize_gpu(
    &mut self,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
    new_byte_size: u64,
  ) {
    self.buffer = resize_impl(&self.buffer, encoder, device, new_byte_size as u32);
  }
  fn byte_size(&self) -> u64 {
    self.buffer.view_byte_size().into()
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    Some(self.buffer.clone())
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    queue.write_buffer(self.buffer.resource.gpu(), offset, content);
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let ty = self.ty.clone().into_shader_single_ty();
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ty),
      writeable_if_storage: !self.readonly,
    };
    let node = bind_builder.binding_dyn(desc).using();
    Box::new(node)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind_dyn(self.buffer.get_binding_build_source());
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    assert_eq!(self.buffer.desc.offset, 0);
    let target = target.get_gpu_buffer_view().unwrap();
    encoder.copy_buffer_to_buffer(
      &self.buffer.buffer.gpu,
      self_offset,
      target.resource.gpu(),
      target_offset + target.desc.offset,
      count,
    );
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl AbstractShaderBindingSource for BoxedAbstractBuffer {
  type ShaderBindResult = BoxedShaderPtr;

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    AbstractBuffer::bind_shader(self, ctx)
  }
}
impl AbstractBindingSource for BoxedAbstractBuffer {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    AbstractBuffer::bind_pass(self, ctx)
  }
}

pub struct AbstractStorageBuffer<T: ?Sized> {
  phantom: PhantomData<T>,
  buffer: BoxedAbstractBuffer,
}

impl<T: ?Sized> Deref for AbstractStorageBuffer<T> {
  type Target = BoxedAbstractBuffer;

  fn deref(&self) -> &Self::Target {
    &self.buffer
  }
}

impl<T: ?Sized + ShaderAbstractPtrAccess> AbstractShaderBindingSource for AbstractStorageBuffer<T> {
  type ShaderBindResult = ShaderPtrOf<T>;

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    let ptr = AbstractBuffer::bind_shader(&self.buffer, ctx);
    T::create_view_from_raw_ptr(ptr)
  }
}
impl<T: ?Sized + ShaderAbstractPtrAccess> AbstractBindingSource for AbstractStorageBuffer<T> {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    AbstractBuffer::bind_pass(&self.buffer, ctx);
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
  buffer: BoxedAbstractBuffer,
}

impl<T: ?Sized> Deref for AbstractReadonlyStorageBuffer<T> {
  type Target = BoxedAbstractBuffer;

  fn deref(&self) -> &Self::Target {
    &self.buffer
  }
}

impl<T: ?Sized> Clone for AbstractReadonlyStorageBuffer<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      buffer: self.buffer.clone(),
    }
  }
}

impl<T: ?Sized + ShaderAbstractPtrAccess> AbstractShaderBindingSource
  for AbstractReadonlyStorageBuffer<T>
{
  type ShaderBindResult = ShaderReadonlyPtrOf<T>;

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    let ptr = AbstractBuffer::bind_shader(&self.buffer, ctx);
    T::create_readonly_view_from_raw_ptr(ptr)
  }
}
impl<T: ?Sized + ShaderAbstractPtrAccess> AbstractBindingSource
  for AbstractReadonlyStorageBuffer<T>
{
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    AbstractBuffer::bind_pass(&self.buffer, ctx);
  }
}

impl<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized + 'static> AbstractBuffer
  for StorageBufferDataView<T>
{
  fn byte_size(&self) -> u64 {
    self.view_byte_size().into()
  }
  fn resize_gpu(
    &mut self,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
    new_byte_size: u64,
  ) {
    self.gpu = resize_impl(&self.gpu, encoder, device, new_byte_size as u32);
  }
  fn ref_clone(&self) -> Box<dyn AbstractBuffer> {
    Box::new(self.clone())
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    queue.write_buffer(self.resource.gpu(), offset + self.desc.offset, content);
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    let target = target.get_gpu_buffer_view().unwrap();
    encoder.copy_buffer_to_buffer(
      self.buffer.gpu(),
      self_offset + self.desc.offset,
      target.resource.gpu(),
      target_offset + target.desc.offset,
      count,
    );
  }

  // todo, code reuse
  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let ty = T::maybe_unsized_ty().into_shader_single_ty();
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ty),
      writeable_if_storage: true,
    };
    let node = bind_builder.binding_dyn(desc).using();
    Box::new(node)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(self);
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    self.gpu.clone().into()
  }
}

impl<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized + 'static> AbstractBuffer
  for StorageBufferReadonlyDataView<T>
{
  fn byte_size(&self) -> u64 {
    self.view_byte_size().into()
  }
  fn resize_gpu(
    &mut self,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
    new_byte_size: u64,
  ) {
    self.gpu = resize_impl(&self.gpu, encoder, device, new_byte_size as u32);
  }
  fn ref_clone(&self) -> Box<dyn AbstractBuffer> {
    Box::new(self.clone())
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    queue.write_buffer(self.resource.gpu(), offset + self.desc.offset, content);
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    let target = target.get_gpu_buffer_view().unwrap();
    encoder.copy_buffer_to_buffer(
      self.buffer.gpu(),
      self_offset + self.desc.offset,
      target.resource.gpu(),
      target_offset + target.desc.offset,
      count,
    );
  }

  // todo, code reuse
  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let ty = T::maybe_unsized_ty().into_shader_single_ty();
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ty),
      writeable_if_storage: false,
    };
    let node = bind_builder.binding_dyn(desc).using();
    Box::new(node)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(self);
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    self.gpu.clone().into()
  }
}

#[must_use]
fn resize_impl(
  buffer: &GPUBufferResourceView,
  encoder: &mut GPUCommandEncoder,
  device: &GPUDevice,
  byte_new_size: u32,
) -> GPUBufferResourceView {
  let usage = buffer.resource.desc.usage;
  let new_buffer =
    create_gpu_buffer_zeroed(byte_new_size as u64, usage, device).create_default_view();

  encoder.copy_buffer_to_buffer(
    &buffer.resource.gpu,
    0,
    &new_buffer.resource.gpu,
    0,
    buffer.resource.desc.size.into(),
  );

  new_buffer
}
