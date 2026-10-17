use crate::*;

mod unlit;
pub use unlit::*;

mod mr;
pub use mr::*;

mod sg;
pub use sg::*;

both!(IndirectAbstractMaterialId, u32);

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
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

pub fn use_tex_watcher_with_host<T, TexStorage>(
  cx: &mut QueryGPUHookCx,
  storage: &mut SparseUpdateStorageWithHostBuffer<TexStorage>,
  offset: usize,
) where
  TexStorage: Std430 + ShaderSizedValueNodeType + Default,
  T: TextureWithSamplingForeignKeys,
{
  let tex_offset = std::mem::offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = std::mem::offset_of!(TextureSamplerHandlePair, sampler_handle);

  cx.use_changes::<SceneTexture2dRefOf<T>>()
    .map(|change| change.map_u32_index_or_u32_max())
    .update_storage_array_with_host(cx, storage, offset + tex_offset);

  cx.use_changes::<SceneSamplerRefOf<T>>()
    .map(|change| change.map_u32_index_or_u32_max())
    .update_storage_array_with_host(cx, storage, offset + sam_offset);
}

pub type ResolvedIndirectModelMaterialRendererBox<'a> =
  Box<dyn ResolvedIndirectModelMaterialRenderer + 'a, &'a dyn Allocator>;

/// Resolve the indirect material renderer of a standard model for one kind of material.
///
/// A list of resolvers is also a resolver, it selects the first one responsible for the model.
pub trait IndirectModelMaterialRendererResolver {
  /// see [IndirectModelRendererResolver::resolve]
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelMaterialRendererBox<'a>, SceneModelRenderDataError>>;
}

impl IndirectModelMaterialRendererResolver for [Box<dyn IndirectModelMaterialRendererResolver>] {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelMaterialRendererBox<'a>, SceneModelRenderDataError>> {
    self
      .iter()
      .find_map(|resolver| resolver.resolve(idx, alloc))
  }
}

impl IndirectModelMaterialRendererResolver for Vec<Box<dyn IndirectModelMaterialRendererResolver>> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelMaterialRendererBox<'a>, SceneModelRenderDataError>> {
    self.as_slice().resolve(idx, alloc)
  }
}

/// The indirect material renderer resolved for one standard model.
pub trait ResolvedIndirectModelMaterialRenderer {
  /// see [ResolvedIndirectModelRenderer::resolver_type_id]
  fn resolver_type_id(&self) -> TypeId;

  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher);
  fn hash_shader_group_key_with_self_type_info(&self, hasher: &mut PipelineHasher) {
    self.hash_shader_group_key(hasher);
    hasher.hash(self.resolver_type_id());
  }

  fn make_component_indirect<'a>(
    &'a self,
    cx: &'a GPUTextureBindingSystem,
    alloc: &'a dyn Allocator,
  ) -> RenderComponentBox<'a>;
}
