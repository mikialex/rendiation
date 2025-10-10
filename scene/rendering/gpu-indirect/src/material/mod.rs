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

pub fn indirect_sample(
  system: &GPUTextureBindingSystem,
  reg: &SemanticRegistry,
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
  default_value: Node<Vec4<f32>>,
) -> Node<Vec4<f32>> {
  let (r, has_texture) = indirect_sample_enabled(system, reg, handles, uv);
  has_texture.select(r, default_value)
}

pub(super) fn indirect_sample_enabled(
  system: &GPUTextureBindingSystem,
  reg: &SemanticRegistry,
  handles: Node<TextureSamplerHandlePair>,
  uv: Node<Vec2<f32>>,
) -> (Node<Vec4<f32>>, Node<bool>) {
  let binding = system
    .as_indirect_system()
    .expect("indirect texture rendering requires indirect binding system");

  let device_pair = handles.expand();
  let device_pair = (device_pair.texture_handle, device_pair.sampler_handle);

  let has_texture = device_pair.0.not_equals(val(u32::MAX));

  let base_level = binding.compute_base_level(reg, uv, device_pair.0, device_pair.1);

  let r = has_texture.select_branched(
    || binding.sample_texture2d_indirect(reg, device_pair.0, device_pair.1, uv, base_level),
    zeroed_val,
  );

  (r, has_texture)
}

pub fn use_tex_watcher<T, TexStorage>(
  cx: &mut QueryGPUHookCx,
  storage: &mut SparseUpdateStorageBuffer<TexStorage>,
  offset: usize,
) where
  TexStorage: Std430 + ShaderSizedValueNodeType + Default,
  T: TextureWithSamplingForeignKeys,
{
  let tex_offset = std::mem::offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = std::mem::offset_of!(TextureSamplerHandlePair, sampler_handle);

  cx.use_changes::<SceneTexture2dRefOf<T>>()
    .map(|change| change.map_u32_index_or_u32_max())
    .update_storage_array(cx, storage, offset + tex_offset);

  cx.use_changes::<SceneSamplerRefOf<T>>()
    .map(|change| change.map_u32_index_or_u32_max())
    .update_storage_array(cx, storage, offset + sam_offset);
}

pub trait IndirectModelMaterialRenderImpl: Any {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>>;
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()>;
  fn as_any(&self) -> &dyn Any;
  fn hash_shader_group_key_with_self_type_info(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.hash_shader_group_key(any_id, hasher).map(|_| {
      self.as_any().type_id().hash(hasher);
    })
  }
}

impl IndirectModelMaterialRenderImpl for Vec<Box<dyn IndirectModelMaterialRenderImpl>> {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    for provider in self {
      if let Some(com) = provider.make_component_indirect(any_idx, cx) {
        return Some(com);
      }
    }
    None
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    for provider in self {
      if let Some(v) = provider.hash_shader_group_key_with_self_type_info(any_id, hasher) {
        return Some(v);
      }
    }
    None
  }
}
