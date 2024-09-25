use crate::*;

mod flat;
pub use flat::*;
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

  (r, device_pair.0.equals(val(0)))
}

pub fn add_tex_watcher<T, TexUniform>(
  uniform: UniformUpdateContainer<EntityHandle<T::Entity>, TexUniform>,
  offset: usize,
  cx: &GPU,
) -> UniformUpdateContainer<EntityHandle<T::Entity>, TexUniform>
where
  TexUniform: Std140 + Default,
  T: TextureWithSamplingForeignKeys,
{
  let tex_offset = std::mem::offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = std::mem::offset_of!(TextureSamplerHandlePair, sampler_handle);

  let tex = global_watch()
    .watch::<SceneTexture2dRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(0))
    .into_uniform_collection_update(offset + tex_offset, cx);

  let sampler = global_watch()
    .watch::<SceneSamplerRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(0))
    .into_uniform_collection_update(offset + sam_offset, cx);

  uniform.with_source(tex).with_source(sampler)
}
