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
pub struct UnlitMaterialDefaultIndirectRenderImplProvider {
  storages: QueryToken,
  tex_storages: QueryToken,
}
impl QueryBasedFeature<Box<dyn IndirectModelMaterialRenderImpl>>
  for UnlitMaterialDefaultIndirectRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let updater = unlit_material_storage_buffer(cx);
    self.storages = qcx.register_multi_updater(updater);
    self.tex_storages = qcx.register_multi_updater(unlit_material_texture_storages(cx));
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.storages);
    qcx.deregister(&mut self.tex_storages);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectModelMaterialRenderImpl> {
    Box::new(UnlitMaterialDefaultIndirectRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefUnlitMaterial>()
        .read_foreign_key(),
      storages: cx.take_storage_array_buffer(self.storages).unwrap(),
      texture_handles: cx.take_storage_array_buffer(self.tex_storages).unwrap(),
      alpha_mode: global_entity_component_of().read(),
    })
  }
}

struct UnlitMaterialDefaultIndirectRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefUnlitMaterial>,
  storages: StorageBufferReadonlyDataView<[UnlitMaterialStorage]>,
  texture_handles: StorageBufferReadonlyDataView<[UnlitMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<UnlitMaterialAlphaConfig>>,
}

impl IndirectModelMaterialRenderImpl for UnlitMaterialDefaultIndirectRenderImpl {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let m = self.material_access.get(any_idx)?;
    Some(Box::new(UnlitMaterialStorageGPU {
      buffer: self.storages.clone(),
      alpha_mode: self.alpha_mode.get_value(m)?,
      texture_handles: self.texture_handles.clone(),
      binding_sys: cx,
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
  storages: QueryToken,
  tex_storages: QueryToken,
}

impl QueryBasedFeature<PbrMRMaterialDefaultIndirectRenderImpl>
  for PbrMRMaterialDefaultIndirectRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.storages = qcx.register_multi_updater(pbr_mr_material_storages(cx));
    self.tex_storages = qcx.register_multi_updater(pbr_mr_material_tex_storages(cx));
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.storages);
    qcx.deregister(&mut self.tex_storages);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> PbrMRMaterialDefaultIndirectRenderImpl {
    PbrMRMaterialDefaultIndirectRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrMRMaterial>()
        .read_foreign_key(),
      storages: cx.take_storage_array_buffer(self.storages).unwrap(),
      tex_storages: cx.take_storage_array_buffer(self.tex_storages).unwrap(),
      alpha_mode: global_entity_component_of().read(),
    }
  }
}

impl QueryBasedFeature<Box<dyn IndirectModelMaterialRenderImpl>>
  for PbrMRMaterialDefaultIndirectRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    (self as &mut dyn QueryBasedFeature<PbrMRMaterialDefaultIndirectRenderImpl, Context = GPU>)
      .register(qcx, cx);
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    (self as &mut dyn QueryBasedFeature<PbrMRMaterialDefaultIndirectRenderImpl, Context = GPU>)
      .deregister(qcx);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectModelMaterialRenderImpl> {
    Box::new(
      (self as &dyn QueryBasedFeature<PbrMRMaterialDefaultIndirectRenderImpl, Context = GPU>)
        .create_impl(cx),
    )
  }
}

pub struct PbrMRMaterialDefaultIndirectRenderImpl {
  pub material_access: ForeignKeyReadView<StandardModelRefPbrMRMaterial>,
  pub storages: StorageBufferReadonlyDataView<[PhysicalMetallicRoughnessMaterialStorage]>,
  pub tex_storages:
    StorageBufferReadonlyDataView<[PhysicalMetallicRoughnessMaterialTextureHandlesStorage]>,
  pub alpha_mode: ComponentReadView<AlphaModeOf<PbrMRMaterialAlphaConfig>>,
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
  storages: QueryToken,
  tex_storages: QueryToken,
}

impl QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl>
  for PbrSGMaterialDefaultIndirectRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.storages = qcx.register_multi_updater(pbr_sg_material_storages(cx));
    self.tex_storages = qcx.register_multi_updater(pbr_sg_material_tex_storages(cx));
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.storages);
    qcx.deregister(&mut self.tex_storages);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> PbrSGMaterialDefaultIndirectRenderImpl {
    PbrSGMaterialDefaultIndirectRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrSGMaterial>()
        .read_foreign_key(),
      storages: cx.take_storage_array_buffer(self.storages).unwrap(),
      tex_storages: cx.take_storage_array_buffer(self.tex_storages).unwrap(),
      alpha_mode: global_entity_component_of().read(),
    }
  }
}

impl QueryBasedFeature<Box<dyn IndirectModelMaterialRenderImpl>>
  for PbrSGMaterialDefaultIndirectRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    (self as &mut dyn QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl, Context = GPU>)
      .register(qcx, cx);
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    (self as &mut dyn QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl, Context = GPU>)
      .deregister(qcx);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn IndirectModelMaterialRenderImpl> {
    Box::new(
      (self as &dyn QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl, Context = GPU>)
        .create_impl(cx),
    )
  }
}

pub struct PbrSGMaterialDefaultIndirectRenderImpl {
  pub material_access: ForeignKeyReadView<StandardModelRefPbrSGMaterial>,
  pub storages: StorageBufferReadonlyDataView<[PhysicalSpecularGlossinessMaterialStorage]>,
  pub tex_storages:
    StorageBufferReadonlyDataView<[PhysicalSpecularGlossinessMaterialTextureHandlesStorage]>,
  pub alpha_mode: ComponentReadView<AlphaModeOf<PbrSGMaterialAlphaConfig>>,
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
