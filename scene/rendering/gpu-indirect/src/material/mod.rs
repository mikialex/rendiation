use crate::*;

mod unlit;
pub use unlit::*;

mod mr;
pub use mr::*;

mod sg;
pub use sg::*;

both!(IndirectAbstractMaterialId, u32);

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Default, Debug, PartialEq)]
pub struct TextureSamplerHandlePair {
  pub texture_handle: u32,
  pub sampler_handle: u32,
}

pub(super) fn bind_and_sample(
  binding: &GPUTextureBindingSystem,
  reg: &SemanticRegistry,
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
  default_value: Node<Vec4<f32>>,
) -> Node<Vec4<f32>> {
  let (r, has_texture) = bind_and_sample_enabled(binding, reg, handles, uv);
  has_texture.select(r, default_value)
}

pub(super) fn bind_and_sample_enabled(
  binding: &GPUTextureBindingSystem,
  reg: &SemanticRegistry,
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
) -> (Node<Vec4<f32>>, Node<bool>) {
  let binding = binding
    .as_indirect_system()
    .expect("indirect texture rendering requires indirect binding system");

  let device_pair = handles.expand();
  let device_pair = (device_pair.texture_handle, device_pair.sampler_handle);

  let has_texture = device_pair.0.not_equals(val(u32::MAX));

  let r = has_texture.select_branched(
    || binding.sample_texture2d_indirect(reg, device_pair.0, device_pair.1, uv),
    zeroed_val,
  );

  (r, has_texture)
}

pub fn add_tex_watcher<T, TexStorage>(
  storage: ReactiveStorageBufferContainer<TexStorage>,
  offset: usize,
) -> ReactiveStorageBufferContainer<TexStorage>
where
  TexStorage: Std430 + Default,
  T: TextureWithSamplingForeignKeys,
{
  let tex_offset = std::mem::offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = std::mem::offset_of!(TextureSamplerHandlePair, sampler_handle);

  let tex = global_watch()
    .watch::<SceneTexture2dRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX));

  let sampler = global_watch()
    .watch::<SceneSamplerRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX));

  storage
    .with_source(tex, offset + tex_offset)
    .with_source(sampler, offset + sam_offset)
}
