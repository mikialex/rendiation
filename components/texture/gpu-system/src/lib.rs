// we design this crate to provide an abstraction over different global gpu texture management
// strategy

use rendiation_shader_api::*;
use rendiation_webgpu::*;
use slab::Slab;
pub type Texture2DHandle = u32;
pub type SamplerHandle = u32;

mod system;
use std::sync::Arc;

pub use system::*;

// todo, support runtime size by query client limitation
pub const MAX_TEXTURE_BINDING_ARRAY_LENGTH: usize = 8192;
pub const MAX_SAMPLER_BINDING_ARRAY_LENGTH: usize = 8192;

pub trait AbstractGPUTextureSystemBase {
  fn register_texture(&mut self, t: GPU2DTextureView) -> Texture2DHandle;
  fn deregister_texture(&mut self, t: Texture2DHandle);
  fn register_sampler(&mut self, t: GPUSamplerView) -> SamplerHandle;
  fn deregister_sampler(&mut self, t: SamplerHandle);
  fn maintain(&mut self);
}

pub trait AbstractTraditionalTextureSystem {
  fn bind_texture2d(&mut self, collector: &mut BindingBuilder, handle: Texture2DHandle);
  fn bind_sampler(&mut self, collector: &mut BindingBuilder, handle: SamplerHandle);

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D>;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler>;

  // note, we do not need to provide abstraction over Node<texture> direct sample
}

pub trait AbstractIndirectGPUTextureSystem {
  fn bind_system_self(&mut self, collector: &mut BindingBuilder);
  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder);
  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

#[derive(Default)]
pub struct TraditionalPerDrawBindingSystem {
  textures: Slab<GPU2DTextureView>,
  samplers: Slab<GPUSamplerView>,
}

impl AbstractGPUTextureSystemBase for TraditionalPerDrawBindingSystem {
  fn register_texture(&mut self, t: GPU2DTextureView) -> Texture2DHandle {
    self.textures.insert(t) as u32
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.textures.remove(t as usize);
  }
  fn register_sampler(&mut self, t: GPUSamplerView) -> SamplerHandle {
    self.samplers.insert(t) as u32
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.samplers.remove(t as usize);
  }
  fn maintain(&mut self) {}
}

impl AbstractTraditionalTextureSystem for TraditionalPerDrawBindingSystem {
  fn bind_texture2d(&mut self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
    let texture = self.textures.get(handle as usize).unwrap();
    collector.bind(texture);
  }

  fn bind_sampler(&mut self, collector: &mut BindingBuilder, handle: SamplerHandle) {
    let sampler = self.samplers.get(handle as usize).unwrap();
    collector.bind(sampler);
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    let texture = self.textures.get(handle as usize).unwrap();
    builder.bind_by(texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    let sampler = self.samplers.get(handle as usize).unwrap();
    builder.bind_by(sampler)
  }
}

pub struct BindlessTextureSystem {
  inner: TraditionalPerDrawBindingSystem,
  texture_binding_array: BindingResourceArray<GPU2DTextureView, MAX_TEXTURE_BINDING_ARRAY_LENGTH>,
  sampler_binding_array: BindingResourceArray<GPUSamplerView, MAX_SAMPLER_BINDING_ARRAY_LENGTH>,
  any_changed: bool, // should we add change mark to per type?
  enable_bindless: bool,
}
impl BindlessTextureSystem {
  pub fn new(enable_bindless: bool) -> Self {
    Self {
      inner: Default::default(),
      texture_binding_array: Default::default(),
      sampler_binding_array: Default::default(),
      any_changed: true,
      enable_bindless,
    }
  }
}

/// pass through inner implementation
impl AbstractGPUTextureSystemBase for BindlessTextureSystem {
  fn register_texture(&mut self, t: GPU2DTextureView) -> Texture2DHandle {
    self.any_changed = true;
    self.inner.register_texture(t)
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.any_changed = true;
    self.inner.deregister_texture(t)
  }
  fn register_sampler(&mut self, t: GPUSamplerView) -> SamplerHandle {
    self.any_changed = true;
    self.inner.register_sampler(t)
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.any_changed = true;
    self.inner.deregister_sampler(t)
  }
  fn maintain(&mut self) {
    if !self.any_changed {
      return;
    }
    self.any_changed = false;
    self.inner.maintain();

    if !self.enable_bindless {
      return;
    }

    // this is not good, maybe we should impl slab by ourself?
    fn slab_to_vec<T: Clone>(s: &Slab<T>) -> Vec<T> {
      let mut r = Vec::with_capacity(s.capacity());
      let default = s.get(0).unwrap();
      s.iter().for_each(|(idx, v)| {
        while idx >= r.len() {
          r.push(default.clone())
        }
        r[idx] = v.clone();
      });
      r
    }

    let source = slab_to_vec(&self.inner.samplers);
    self.sampler_binding_array = BindingResourceArray::<
      GPUSamplerView,
      MAX_TEXTURE_BINDING_ARRAY_LENGTH,
    >::new(Arc::new(source));

    let source = slab_to_vec(&self.inner.textures);
    self.texture_binding_array = BindingResourceArray::<
      GPU2DTextureView,
      MAX_TEXTURE_BINDING_ARRAY_LENGTH,
    >::new(Arc::new(source));
  }
}

impl AbstractTraditionalTextureSystem for BindlessTextureSystem {
  fn bind_texture2d(&mut self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
    self.inner.bind_texture2d(collector, handle)
  }

  fn bind_sampler(&mut self, collector: &mut BindingBuilder, handle: SamplerHandle) {
    self.inner.bind_sampler(collector, handle)
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    self.inner.register_shader_texture2d(builder, handle)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    self.inner.register_shader_sampler(builder, handle)
  }
}
both!(
  BindlessTexturesInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderTexture2D>, MAX_TEXTURE_BINDING_ARRAY_LENGTH>>
);
both!(
  BindlessSamplersInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderSampler>, MAX_SAMPLER_BINDING_ARRAY_LENGTH>>
);

impl AbstractIndirectGPUTextureSystem for BindlessTextureSystem {
  fn bind_system_self(&mut self, collector: &mut BindingBuilder) {
    collector.bind(&self.texture_binding_array);
    collector.bind(&self.sampler_binding_array);
  }

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder
      .bind_by(&self.texture_binding_array)
      .using_graphics_pair(builder, |r, textures| {
        r.register_typed_both_stage::<BindlessTexturesInShader>(textures);
      });
    builder
      .bind_by(&self.sampler_binding_array)
      .using_graphics_pair(builder, |r, samplers| {
        r.register_typed_both_stage::<BindlessSamplersInShader>(samplers);
      });
  }

  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let textures = reg
      .query_typed_both_stage::<BindlessTexturesInShader>()
      .unwrap();

    let samplers = reg
      .query_typed_both_stage::<BindlessSamplersInShader>()
      .unwrap();

    let texture = textures.index(shader_texture_handle);
    let sampler = samplers.index(shader_sampler_handle);
    texture.sample(sampler, uv)
  }
}
