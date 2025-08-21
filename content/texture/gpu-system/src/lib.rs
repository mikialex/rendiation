// This crate is to provide an abstraction over different global gpu texture management
// strategy and implementation.

use dyn_clone::DynClone;
use rendiation_shader_api::*;
use rendiation_texture_gpu_base::*;
use rendiation_webgpu::*;
pub type Texture2DHandle = u32;
pub type SamplerHandle = u32;

mod bindless;
pub use bindless::*;
mod gles;
pub use gles::*;
mod pool;
use std::sync::Arc;
use std::{any::Any, hash::Hash};

pub use pool::*;
use query::*;

pub trait AbstractIndirectGPUTextureSystem {
  fn bind_system_self(&self, collector: &mut BindingBuilder);
  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder);
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  );
  /// caller must ensure the texture and sample handle are valid
  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

pub trait AbstractGPUTextureSystem: Clone {
  type RegisteredShaderTexture: Copy + 'static;
  type RegisteredShaderSampler: Copy + 'static;

  fn bind_system_self(&self, collector: &mut BindingBuilder);
  fn bind_texture2d(&self, collector: &mut BindingBuilder, handle: Texture2DHandle);
  fn bind_sampler(&self, collector: &mut BindingBuilder, handle: SamplerHandle);

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder);
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  );
  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle_host: Texture2DHandle,
    handle_device: Node<Texture2DHandle>,
  ) -> Self::RegisteredShaderTexture;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle_host: SamplerHandle,
    handle_device: Node<SamplerHandle>,
  ) -> Self::RegisteredShaderSampler;

  fn sample_texture2d(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Self::RegisteredShaderTexture,
    shader_sampler_handle: Self::RegisteredShaderSampler,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;

  fn sample_texture2d_with_shader_bind(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    reg: &SemanticRegistry,
    host_handles: (Texture2DHandle, SamplerHandle),
    device_handles: (Node<Texture2DHandle>, Node<SamplerHandle>),
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = self.register_shader_texture2d(binding, host_handles.0, device_handles.0);
    let sampler = self.register_shader_sampler(binding, host_handles.1, device_handles.1);
    self.sample_texture2d(reg, texture, sampler, uv)
  }

  /// if implementation not support, return None
  fn as_indirect_system(&self) -> Option<&dyn AbstractIndirectGPUTextureSystem>;
}

impl<T: AbstractIndirectGPUTextureSystem + Clone> AbstractGPUTextureSystem for T {
  type RegisteredShaderTexture = Node<Texture2DHandle>;
  type RegisteredShaderSampler = Node<SamplerHandle>;

  fn bind_system_self(&self, collector: &mut BindingBuilder) {
    self.bind_system_self(collector)
  }

  fn bind_texture2d(&self, _: &mut BindingBuilder, _: Texture2DHandle) {}
  fn bind_sampler(&self, _: &mut BindingBuilder, _: SamplerHandle) {}

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.register_system_self(builder)
  }
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) {
    self.register_system_self_for_compute(builder, reg)
  }

  fn register_shader_texture2d(
    &self,
    _: &mut ShaderBindGroupBuilder,
    _: Texture2DHandle,
    handle_device: Node<Texture2DHandle>,
  ) -> Self::RegisteredShaderTexture {
    handle_device
  }

  fn register_shader_sampler(
    &self,
    _: &mut ShaderBindGroupBuilder,
    _: SamplerHandle,
    handle_device: Node<SamplerHandle>,
  ) -> Self::RegisteredShaderSampler {
    handle_device
  }

  fn sample_texture2d(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Self::RegisteredShaderTexture,
    shader_sampler_handle: Self::RegisteredShaderSampler,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    self.sample_texture2d_indirect(reg, shader_texture_handle, shader_sampler_handle, uv)
  }

  fn as_indirect_system(&self) -> Option<&dyn AbstractIndirectGPUTextureSystem> {
    Some(self)
  }
}

/// the object safe version of [AbstractGPUTextureSystem]
pub trait DynAbstractGPUTextureSystem: Any + DynClone {
  fn bind_system_self(&self, collector: &mut BindingBuilder);
  fn bind_texture2d(&self, collector: &mut BindingBuilder, handle: Texture2DHandle);
  fn bind_sampler(&self, collector: &mut BindingBuilder, handle: SamplerHandle);

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder);
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  );
  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle_host: Texture2DHandle,
    handle_device: Node<Texture2DHandle>,
  ) -> Box<dyn Any>;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle_host: SamplerHandle,
    handle_device: Node<SamplerHandle>,
  ) -> Box<dyn Any>;

  fn sample_texture2d(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Box<dyn Any>,
    shader_sampler_handle: Box<dyn Any>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;

  fn sample_texture2d_with_shader_bind(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    reg: &SemanticRegistry,
    host_handles: (Texture2DHandle, SamplerHandle),
    device_handles: (Node<Texture2DHandle>, Node<SamplerHandle>),
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = self.register_shader_texture2d(binding, host_handles.0, device_handles.0);
    let sampler = self.register_shader_sampler(binding, host_handles.1, device_handles.1);
    self.sample_texture2d(reg, texture, sampler, uv)
  }
  fn as_indirect_system(&self) -> Option<&dyn AbstractIndirectGPUTextureSystem>;
}
dyn_clone::clone_trait_object!(DynAbstractGPUTextureSystem);

impl<T: AbstractGPUTextureSystem + Any> DynAbstractGPUTextureSystem for T {
  fn bind_system_self(&self, collector: &mut BindingBuilder) {
    self.bind_system_self(collector)
  }
  fn bind_texture2d(&self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
    self.bind_texture2d(collector, handle)
  }
  fn bind_sampler(&self, collector: &mut BindingBuilder, handle: SamplerHandle) {
    self.bind_sampler(collector, handle)
  }
  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.register_system_self(builder)
  }
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) {
    self.register_system_self_for_compute(builder, reg)
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle_host: Texture2DHandle,
    handle_device: Node<Texture2DHandle>,
  ) -> Box<dyn Any> {
    Box::new(self.register_shader_texture2d(builder, handle_host, handle_device))
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle_host: SamplerHandle,
    handle_device: Node<SamplerHandle>,
  ) -> Box<dyn Any> {
    Box::new(self.register_shader_sampler(builder, handle_host, handle_device))
  }

  fn sample_texture2d(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Box<dyn Any>,
    shader_sampler_handle: Box<dyn Any>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    self.sample_texture2d(
      reg,
      *shader_texture_handle
        .downcast::<T::RegisteredShaderTexture>()
        .unwrap(),
      *shader_sampler_handle
        .downcast::<T::RegisteredShaderSampler>()
        .unwrap(),
      uv,
    )
  }

  fn as_indirect_system(&self) -> Option<&dyn AbstractIndirectGPUTextureSystem> {
    self.as_indirect_system()
  }
}

impl ShaderPassBuilder for Box<dyn DynAbstractGPUTextureSystem> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.bind_system_self(&mut ctx.binding)
  }
}
impl ShaderHashProvider for Box<dyn DynAbstractGPUTextureSystem> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).type_id().hash(hasher);
  }
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for Box<dyn DynAbstractGPUTextureSystem> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.register_system_self(builder);
  }
}
