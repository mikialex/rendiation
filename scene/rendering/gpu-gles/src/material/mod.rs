use std::sync::Arc;

use crate::*;

mod unlit;
pub use unlit::*;
mod mr;
pub use mr::*;
mod sg;
pub use sg::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default, Debug, PartialEq)]
pub struct TextureSamplerHandlePair {
  pub texture_handle: u32,
  pub sampler_handle: u32,
}

pub(super) fn setup_tex(
  ctx: &mut GPURenderPassCtx,
  binding_sys: &GPUTextureBindingSystem,
  (tex, sampler): (u32, u32),
) {
  binding_sys.bind_texture2d(&mut ctx.binding, tex);
  binding_sys.bind_sampler(&mut ctx.binding, sampler);
}

pub(super) fn bind_and_sample(
  binding: &GPUTextureBindingSystem,
  builder: &mut ShaderBindGroupBuilder,
  reg: &SemanticRegistry,
  host_pair: (u32, u32),
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
  default_value: Node<Vec4<f32>>,
) -> Node<Vec4<f32>> {
  let (r, has_texture) = bind_and_sample_enabled(binding, builder, reg, host_pair, handles, uv);
  has_texture.select(r, default_value)
}

pub(super) fn bind_and_sample_enabled(
  binding: &GPUTextureBindingSystem,
  builder: &mut ShaderBindGroupBuilder,
  reg: &SemanticRegistry,
  host_pair: (u32, u32),
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
) -> (Node<Vec4<f32>>, Node<bool>) {
  let device_pair = handles.expand();
  let device_pair = (device_pair.texture_handle, device_pair.sampler_handle);
  let r = binding.sample_texture2d_with_shader_bind(builder, reg, host_pair, device_pair, uv);

  (r, device_pair.0.not_equals(val(u32::MAX)))
}

pub fn use_tex_watcher<T, TexUniform>(
  cx: &mut GPUResourceCx<'_>,
  offset: usize,
  uniform: &UniformBufferCollection<EntityHandle<T::Entity>, TexUniform>,
) where
  TexUniform: Std140 + Default,
  T: TextureWithSamplingForeignKeys,
{
  let tex_offset = std::mem::offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = std::mem::offset_of!(TextureSamplerHandlePair, sampler_handle);

  cx.use_changes::<SceneTexture2dRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .update_uniforms(uniform, offset + tex_offset);

  cx.use_changes::<SceneSamplerRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .update_uniforms(uniform, offset + sam_offset);
}

pub trait GLESModelMaterialRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
}

impl GLESModelMaterialRenderImpl for Vec<Box<dyn GLESModelMaterialRenderImpl>> {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(com) = provider.make_component(idx, cx) {
        return Some(com);
      }
    }
    None
  }
}

pub struct TextureSamplerIdView<T: TextureWithSamplingForeignKeys> {
  pub texture: ForeignKeyReadView<SceneTexture2dRefOf<T>>,
  pub sampler: ForeignKeyReadView<SceneSamplerRefOf<T>>,
}

impl<T: TextureWithSamplingForeignKeys> TextureSamplerIdView<T> {
  pub fn read_from_global() -> Self {
    Self {
      texture: global_entity_component_of().read_foreign_key(),
      sampler: global_entity_component_of().read_foreign_key(),
    }
  }

  pub fn get_pair(&self, id: EntityHandle<T::Entity>) -> Option<(u32, u32)> {
    let tex = self.texture.get(id)?;
    let tex = tex.alloc_index();
    let sampler = self.sampler.get(id)?;
    let sampler = sampler.alloc_index();
    Some((tex, sampler))
  }
}

pub const EMPTY_H: (u32, u32) = (u32::MAX, u32::MAX);
