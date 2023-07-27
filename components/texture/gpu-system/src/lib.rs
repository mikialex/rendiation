// we could not depend on shadergraph theoretically if we abstract over shader node compose
// but that will too complicated
use shadergraph::*;
pub type Texture2DHandle = u32;
pub type SamplerHandle = u32;

pub trait GPUTextureBackend {
  type GPUTexture2D: ShaderBindingProvider<Node = ShaderTexture2D>;
  type GPUSampler: ShaderBindingProvider<Node = ShaderSampler>;

  type BindingCollector;
  fn bind_texture2d(collector: &mut Self::BindingCollector, texture: &Self::GPUTexture2D);
  fn bind_sampler(collector: &mut Self::BindingCollector, texture: &Self::GPUSampler);
}

pub trait GPUTextureAdvanceBackend: GPUTextureBackend {
  type GPUStorageBuffer<T>: ShaderBindingProvider<Node = T>;

  fn bind_storage<T>(collector: &mut Self::BindingCollector, buffer: &Self::GPUStorageBuffer<T>);
}

pub trait AbstractGPUTextureSystemBase<B: GPUTextureBackend> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle;
  fn deregister_texture(&mut self, t: Texture2DHandle);
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle;
  fn deregister_sampler(&mut self, t: SamplerHandle);
}

pub trait AbstractTraditionalTextureSystem<B: GPUTextureBackend> {
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle);
  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle);

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D>;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler>;

  // note, we do not need to provide abstraction over Node<texture> direct sample
}

pub trait AbstractIndirectGPUTextureSystem<B: GPUTextureBackend> {
  fn bind_system_self(&mut self, collector: &mut B::BindingCollector);
  fn register_system_self(&self, builder: &mut ShaderGraphRenderPipelineBuilder);
  fn sample_texture2d_indirect(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

pub trait SlabAllocator<T> {
  fn allocate(&mut self, item: T) -> u32;
  fn deallocate(&mut self, item: u32);
  fn get(&self, handle: u32) -> &T;
}

pub struct TraditionalPerDrawBindingSystem<B: GPUTextureBackend> {
  textures: Box<dyn SlabAllocator<B::GPUTexture2D>>,
  samplers: Box<dyn SlabAllocator<B::GPUSampler>>,
}

impl<B: GPUTextureBackend> AbstractGPUTextureSystemBase<B> for TraditionalPerDrawBindingSystem<B> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle {
    self.textures.allocate(t)
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.textures.deallocate(t)
  }
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle {
    self.samplers.allocate(t)
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.samplers.deallocate(t)
  }
}

impl<B: GPUTextureBackend> AbstractTraditionalTextureSystem<B>
  for TraditionalPerDrawBindingSystem<B>
{
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle) {
    let texture = self.textures.get(handle);
    B::bind_texture2d(collector, texture);
  }

  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle) {
    let sampler = self.samplers.get(handle);
    B::bind_sampler(collector, sampler);
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D> {
    let texture = self.textures.get(handle);
    builder.uniform_by(texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler> {
    let sampler = self.samplers.get(handle);
    builder.uniform_by(sampler)
  }
}

pub struct BindlessTextureSystem<B: GPUTextureBackend> {
  inner: TraditionalPerDrawBindingSystem<B>,
}

/// pass through inner implementation
impl<B: GPUTextureBackend> AbstractTraditionalTextureSystem<B> for BindlessTextureSystem<B> {
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle) {
    self.inner.bind_texture2d(collector, handle)
  }

  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle) {
    self.inner.bind_sampler(collector, handle)
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D> {
    self.inner.register_shader_texture2d(builder, handle)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler> {
    self.inner.register_shader_sampler(builder, handle)
  }
}

impl<B: GPUTextureBackend> AbstractIndirectGPUTextureSystem<B> for BindlessTextureSystem<B> {
  fn bind_system_self(&mut self, _collector: &mut B::BindingCollector) {
    todo!()
  }

  fn register_system_self(&self, _builder: &mut ShaderGraphRenderPipelineBuilder) {
    todo!()
  }

  fn sample_texture2d_indirect(
    &self,
    _builder: &mut ShaderGraphBindGroupDirectBuilder,
    _shader_texture_handle: Node<Texture2DHandle>,
    _shader_sampler_handle: Node<SamplerHandle>,
    _uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    todo!()
  }
}

// pub struct VirtualTextureSystem<B: GPUTextureBackend> {
//   great_pool: B::GPUTexture2D,
//   page_textures: Box<dyn SlabAllocator<B::GPUTexture2D>>,
// }
