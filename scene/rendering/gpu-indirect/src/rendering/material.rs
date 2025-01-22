use crate::*;

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

#[derive(Default)]
pub struct FlatMaterialDefaultIndirectRenderImplProvider {
  storages: UpdateResultToken,
}
impl RenderImplProvider<Box<dyn IndirectModelMaterialRenderImpl>>
  for FlatMaterialDefaultIndirectRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let updater = flat_material_storage_buffer(cx);
    self.storages = source.register_multi_updater(updater.inner);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.storages);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn IndirectModelMaterialRenderImpl> {
    Box::new(FlatMaterialDefaultIndirectRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefFlatMaterial>()
        .read_foreign_key(),
      storages: res
        .take_multi_updater_updated::<CommonStorageBufferImpl<FlatMaterialStorage>>(self.storages)
        .unwrap()
        .inner
        .gpu()
        .clone(),
      alpha_mode: global_entity_component_of().read(),
    })
  }
}

struct FlatMaterialDefaultIndirectRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefFlatMaterial>,
  storages: StorageBufferReadOnlyDataView<[FlatMaterialStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<FlatMaterialAlphaConfig>>,
}

impl IndirectModelMaterialRenderImpl for FlatMaterialDefaultIndirectRenderImpl {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let m = self.material_access.get(any_idx)?;
    Some(Box::new(FlatMaterialStorageGPU {
      buffer: self.storages.clone(),
      alpha_mode: self.alpha_mode.get_value(m)?,
    }))
  }
  fn hash_shader_group_key(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
    _: &mut PipelineHasher,
  ) -> Option<()> {
    self.material_access.get(any_idx)?;
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[derive(Default)]
pub struct PbrMRMaterialDefaultIndirectRenderImplProvider {
  storages: UpdateResultToken,
  tex_storages: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn IndirectModelMaterialRenderImpl>>
  for PbrMRMaterialDefaultIndirectRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.storages = source.register_multi_updater(pbr_mr_material_storages(cx).inner);
    self.tex_storages = source.register_multi_updater(pbr_mr_material_tex_storages(cx).inner);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.storages);
    source.deregister(&mut self.tex_storages);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn IndirectModelMaterialRenderImpl> {
    Box::new(PbrMRMaterialDefaultIndirectRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrMRMaterial>()
        .read_foreign_key(),
      storages: res.take_multi_updater_updated::<CommonStorageBufferImpl<PhysicalMetallicRoughnessMaterialStorage>>(self.storages).unwrap().target.gpu().clone(),
      tex_storages: res.take_multi_updater_updated::<CommonStorageBufferImpl<PhysicalMetallicRoughnessMaterialTextureHandlesStorage>>(self.tex_storages).unwrap().target.gpu().clone(),
      alpha_mode: global_entity_component_of().read(),
    })
  }
}

struct PbrMRMaterialDefaultIndirectRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefPbrMRMaterial>,
  storages: StorageBufferReadOnlyDataView<[PhysicalMetallicRoughnessMaterialStorage]>,
  tex_storages:
    StorageBufferReadOnlyDataView<[PhysicalMetallicRoughnessMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrMRMaterialAlphaConfig>>,
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

impl IndirectModelMaterialRenderImpl for PbrMRMaterialDefaultIndirectRenderImpl {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(any_idx)?;
    let r = PhysicalMetallicRoughnessMaterialIndirectGPU {
      storage: &self.storages,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      texture_storages: &self.tex_storages,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
  fn hash_shader_group_key(
    &self,
    idx: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let idx = self.material_access.get(idx)?;
    self.alpha_mode.get_value(idx)?.hash(hasher);
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[derive(Default)]
pub struct PbrSGMaterialDefaultIndirectRenderImplProvider {
  storages: UpdateResultToken,
  tex_storages: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn IndirectModelMaterialRenderImpl>>
  for PbrSGMaterialDefaultIndirectRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.storages = source.register_multi_updater(pbr_sg_material_storages(cx).inner);
    self.tex_storages = source.register_multi_updater(pbr_sg_material_tex_storages(cx).inner);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.storages);
    source.deregister(&mut self.tex_storages);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn IndirectModelMaterialRenderImpl> {
    Box::new(PbrSGMaterialDefaultIndirectRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrSGMaterial>()
        .read_foreign_key(),
      storages: res.take_multi_updater_updated::<CommonStorageBufferImpl<PhysicalSpecularGlossinessMaterialStorage>>(self.storages).unwrap().target.gpu().clone(),
      tex_storages: res.take_multi_updater_updated::<CommonStorageBufferImpl<PhysicalSpecularGlossinessMaterialTextureHandlesStorage>>(self.tex_storages).unwrap().target.gpu().clone(),
      alpha_mode: global_entity_component_of().read(),
    })
  }
}

struct PbrSGMaterialDefaultIndirectRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefPbrSGMaterial>,
  storages: StorageBufferReadOnlyDataView<[PhysicalSpecularGlossinessMaterialStorage]>,
  tex_storages:
    StorageBufferReadOnlyDataView<[PhysicalSpecularGlossinessMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrSGMaterialAlphaConfig>>,
}

impl IndirectModelMaterialRenderImpl for PbrSGMaterialDefaultIndirectRenderImpl {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(any_idx)?;
    let r = PhysicalSpecularGlossinessMaterialGPU {
      storage: &self.storages,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      texture_storages: &self.tex_storages,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
  fn hash_shader_group_key(
    &self,
    idx: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let idx = self.material_access.get(idx)?;
    self.alpha_mode.get_value(idx)?.hash(hasher);
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}
