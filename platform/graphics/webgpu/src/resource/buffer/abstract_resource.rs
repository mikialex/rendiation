use crate::*;

pub trait AbstractStorageAllocator: DynClone + Send + Sync {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
    label: Option<&str>,
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
    label: Option<&str>,
  ) -> BoxedAbstractBuffer {
    (**self).allocate_dyn_ty(byte_size, device, ty_desc, readonly, label)
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
    label: Option<&str>,
  ) -> BoxedAbstractBuffer {
    (**self).allocate_dyn_ty(byte_size, device, ty_desc, readonly, label)
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
    label: Option<&str>,
  ) -> AbstractStorageBuffer<T>;

  /// only valid if the allocator is config readonly
  fn allocate_readonly<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    label: Option<&str>,
  ) -> AbstractReadonlyStorageBuffer<T>;

  fn allocate_readonly_init<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    value: &T,
    gpu: &GPU,
    label: Option<&str>,
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
    label: Option<&str>,
  ) -> AbstractStorageBuffer<T> {
    AbstractStorageBuffer {
      buffer: self.allocate_dyn_ty(byte_size, device, T::maybe_unsized_ty(), false, label),
      phantom: Default::default(),
    }
  }

  fn allocate_readonly<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    label: Option<&str>,
  ) -> AbstractReadonlyStorageBuffer<T> {
    AbstractReadonlyStorageBuffer {
      buffer: self.allocate_dyn_ty(byte_size, device, T::maybe_unsized_ty(), true, label),
      phantom: Default::default(),
    }
  }

  fn allocate_readonly_init<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    value: &T,
    gpu: &GPU,
    label: Option<&str>,
  ) -> AbstractReadonlyStorageBuffer<T> {
    let value = value.bytes();

    let buffer = self.allocate_dyn_ty(
      value.len() as u64,
      &gpu.device,
      T::maybe_unsized_ty(),
      true,
      label,
    );

    buffer.write(value, 0, &gpu.queue);

    AbstractReadonlyStorageBuffer {
      buffer,
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
    label: Option<&str>,
  ) -> BoxedAbstractBuffer {
    // this ty mark and read_write mark is useless actually
    let init = StorageBufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap());
    let buffer = StorageBufferReadonlyDataView::<[u32]>::create_by(device, label, init).gpu;
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

pub struct BufferRelocate {
  pub self_offset: u64,
  pub target_offset: u64,
  pub count: u64,
}

/// the clone trait implements ref clone semantic
pub trait AbstractBuffer: DynClone + Send + Sync {
  fn byte_size(&self) -> u64;
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice, new_byte_size: u64);
  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue);

  /// as the abstract buffer not able to deep clone it self.
  /// we use this api to express the self may overlapping batch relocate logic.
  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  );

  /// the target must be a different buffer(not ref cloned self)
  ///
  /// The target must be the same type of self
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

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) {
    (**self).batch_self_relocate(iter, encoder, device)
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    (**self).get_gpu_buffer_view()
  }

  fn as_any(&self) -> &dyn Any {
    (**self).as_any()
  }
}

fn batch_relocate_impl(
  self_buffer: &GPUBufferResourceView,
  encoder: &mut GPUCommandEncoder,
  device: &GPUDevice,
  iter: &mut dyn Iterator<Item = BufferRelocate>,
) {
  let byte_size_self: u64 = self_buffer.view_byte_size().into();
  // todo, we could reduce the copy size by checking relocation bound
  let buffer_source = create_gpu_buffer_zeroed(
    byte_size_self,
    BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    device,
  );
  encoder.copy_buffer_to_buffer(
    self_buffer.resource.gpu(),
    self_buffer.desc.offset,
    buffer_source.resource.gpu(),
    0,
    byte_size_self,
  );

  iter.for_each(|relocate| {
    encoder.copy_buffer_to_buffer(
      buffer_source.resource.gpu(),
      relocate.self_offset,
      self_buffer.resource.gpu(),
      self_buffer.desc.offset + relocate.target_offset,
      relocate.count,
    );
  });
}

#[derive(Clone)]
pub struct DynTypedStorageBuffer {
  pub buffer: GPUBufferResourceView,
  pub ty: MaybeUnsizedValueType,
  pub readonly: bool,
}
impl AbstractBuffer for DynTypedStorageBuffer {
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
    let target = target.get_gpu_buffer_view().unwrap();
    encoder.copy_buffer_to_buffer(
      &self.buffer.buffer.gpu,
      self_offset + self.buffer.desc.offset,
      target.resource.gpu(),
      target_offset + target.desc.offset,
      count,
    );
  }

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) {
    batch_relocate_impl(&self.buffer, encoder, device, iter);
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
  pub(crate) buffer: BoxedAbstractBuffer,
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
  pub(crate) buffer: BoxedAbstractBuffer,
}

impl<T: Std430> AbstractReadonlyStorageBuffer<[T]> {
  pub fn item_count(&self) -> u32 {
    (self.buffer.byte_size() / std::mem::size_of::<T>() as u64) as u32
  }
}

impl<T: ?Sized + Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType>
  From<StorageBufferReadonlyDataView<T>> for AbstractReadonlyStorageBuffer<T>
{
  fn from(value: StorageBufferReadonlyDataView<T>) -> Self {
    Self {
      phantom: Default::default(),
      buffer: Box::new(DynTypedStorageBuffer {
        buffer: value.gpu,
        ty: T::maybe_unsized_ty(),
        readonly: true,
      }),
    }
  }
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

impl<T> AbstractBuffer for StorageBufferDataView<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized + 'static,
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

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) {
    batch_relocate_impl(self, encoder, device, iter);
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

impl<T> AbstractBuffer for StorageBufferReadonlyDataView<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized + 'static,
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

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) {
    batch_relocate_impl(self, encoder, device, iter);
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
    Some(buffer.resource.desc.size.into()),
  );

  new_buffer
}

impl<T> AbstractBuffer for AbstractReadonlyStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized + 'static,
{
  fn byte_size(&self) -> u64 {
    self.buffer.byte_size()
  }

  fn resize_gpu(
    &mut self,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
    new_byte_size: u64,
  ) {
    self.buffer.resize_gpu(encoder, device, new_byte_size);
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    self.buffer.write(content, offset, queue);
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    self
      .buffer
      .copy_buffer_to_buffer(target, self_offset, target_offset, count, encoder);
  }

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    encoder: &mut GPUCommandEncoder,
    device: &GPUDevice,
  ) {
    self.buffer.batch_self_relocate(iter, encoder, device);
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    AbstractBuffer::bind_shader(&self.buffer, bind_builder)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    AbstractBuffer::bind_pass(&self.buffer, bind_builder)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    self.buffer.get_gpu_buffer_view()
  }
}
