use crate::*;

pub trait AbstractStorageAllocator {
  fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> BoxedAbstractStorageBuffer<T>;

  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedAbstractStorageBufferDynTyped;
}

pub struct DefaultStorageAllocator;
impl AbstractStorageAllocator for DefaultStorageAllocator {
  fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> BoxedAbstractStorageBuffer<T> {
    Box::new(create_gpu_read_write_storage::<T>(
      StorageBufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap()),
      &device,
    ))
  }

  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedAbstractStorageBufferDynTyped {
    // this ty mark is useless actually
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
}

pub type BoxedAbstractStorageBuffer<T> = Box<dyn AbstractStorageBuffer<T>>;
pub trait AbstractStorageBuffer<T>: DynClone
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView;
  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue);
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T>;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}
impl<T> Clone for BoxedAbstractStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}
impl<T> AbstractStorageBuffer<T> for BoxedAbstractStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
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
  ) -> ShaderPtrOf<T> {
    (**self).bind_shader(bind_builder, registry)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    (**self).bind_pass(bind_builder)
  }
}

impl<T> AbstractStorageBuffer<T> for StorageBufferDataView<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    self.resource.create_default_view()
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    queue.write_buffer(self.resource.gpu(), offset, content);
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    bind_builder.bind_by(self)
  }
  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(self);
  }
}

pub type BoxedAbstractUniformBuffer<T> = Box<dyn AbstractUniformBuffer<T>>;
pub trait AbstractUniformBuffer<T>: DynClone
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView;
  fn write(&self, content: &[u8], offset: u64, _queue: &GPUQueue);
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T>;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}
impl<T> Clone for BoxedAbstractUniformBuffer<T> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}
impl<T> AbstractUniformBuffer<T> for BoxedAbstractUniformBuffer<T>
where
  T: ShaderSizedValueNodeType + Std140,
{
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
  ) -> ShaderReadonlyPtrOf<T> {
    (**self).bind_shader(bind_builder, registry)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    (**self).bind_pass(bind_builder)
  }
}

impl<T> AbstractUniformBuffer<T> for UniformBufferDataView<T>
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    self.gpu.clone()
  }

  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue) {
    queue.write_buffer(self.gpu.resource.gpu(), offset, content);
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T> {
    bind_builder.bind_by(self)
  }
  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(self);
  }
}

pub type BoxedAbstractStorageBufferDynTyped = Box<dyn AbstractStorageBufferDynTyped>;
pub trait AbstractStorageBufferDynTyped: DynClone {
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView;
  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue);
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> BoxedShaderPtr;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(AbstractStorageBufferDynTyped);
impl AbstractStorageBufferDynTyped for BoxedAbstractStorageBufferDynTyped {
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
impl AbstractStorageBufferDynTyped for DynTypedStorageBuffer {
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
    buffer: &dyn AbstractStorageBufferDynTyped,
  ) -> BoxedShaderPtr;
  fn bind_abstract_storage<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    buffer: &impl AbstractStorageBuffer<T>,
  ) -> ShaderPtrOf<T>;
  fn bind_abstract_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    buffer: &impl AbstractUniformBuffer<T>,
  ) -> ShaderReadonlyPtrOf<T>;
}
impl ComputeShaderBuilderAbstractBufferExt for ShaderComputePipelineBuilder {
  fn bind_abstract_storage_dyn_typed(
    &mut self,
    buffer: &dyn AbstractStorageBufferDynTyped,
  ) -> BoxedShaderPtr {
    buffer.bind_shader(&mut self.bindgroups, &mut self.registry)
  }
  fn bind_abstract_storage<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    buffer: &impl AbstractStorageBuffer<T>,
  ) -> ShaderPtrOf<T> {
    buffer.bind_shader(&mut self.bindgroups, &mut self.registry)
  }

  fn bind_abstract_uniform<T>(
    &mut self,
    buffer: &impl AbstractUniformBuffer<T>,
  ) -> ShaderReadonlyPtrOf<T>
  where
    T: ShaderSizedValueNodeType + Std140,
  {
    buffer.bind_shader(&mut self.bindgroups, &mut self.registry)
  }
}
pub trait BindBuilderAbstractBufferExt: Sized {
  fn bind_abstract_storage_dyn_typed(
    &mut self,
    buffer: &dyn AbstractStorageBufferDynTyped,
  ) -> &mut Self;
  fn bind_abstract_storage<T>(&mut self, buffer: &impl AbstractStorageBuffer<T>) -> &mut Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized;
  fn with_bind_abstract_storage<T>(mut self, buffer: &impl AbstractStorageBuffer<T>) -> Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
  {
    self.bind_abstract_storage(buffer);
    self
  }
  fn bind_abstract_uniform<T>(&mut self, buffer: &impl AbstractUniformBuffer<T>) -> &mut Self
  where
    T: ShaderSizedValueNodeType + Std140;
  fn with_bind_abstract_uniform<T>(mut self, buffer: &impl AbstractUniformBuffer<T>) -> Self
  where
    T: ShaderSizedValueNodeType + Std140,
  {
    self.bind_abstract_uniform(buffer);
    self
  }
}
impl BindBuilderAbstractBufferExt for BindingBuilder {
  fn bind_abstract_storage_dyn_typed(
    &mut self,
    buffer: &dyn AbstractStorageBufferDynTyped,
  ) -> &mut Self {
    buffer.bind_pass(self);
    self
  }
  fn bind_abstract_storage<T>(&mut self, buffer: &impl AbstractStorageBuffer<T>) -> &mut Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
  {
    buffer.bind_pass(self);
    self
  }

  fn bind_abstract_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    buffer: &impl AbstractUniformBuffer<T>,
  ) -> &mut Self {
    buffer.bind_pass(self);
    self
  }
}
