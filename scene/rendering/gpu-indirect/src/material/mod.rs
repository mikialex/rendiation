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
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .into_query_update_storage(offset + tex_offset);

  let sampler = global_watch()
    .watch::<SceneSamplerRefOf<T>>()
    .collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
    .into_query_update_storage(offset + sam_offset);

  storage.with_source(tex).with_source(sampler)
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
