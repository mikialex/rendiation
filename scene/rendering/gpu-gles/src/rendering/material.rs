use crate::*;

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

pub fn use_unlit_material_uniforms(
  cx: &mut QueryGPUHookCx,
) -> Option<UnlitMaterialDefaultRenderImpl> {
  let uniform = cx.use_uniform_buffers::<EntityHandle<UnlitMaterialEntity>, UnlitMaterialUniform>(
    |source, cx| {
      let color = global_watch()
        .watch::<UnlitMaterialColorComponent>()
        .collective_map(srgb4_to_linear4)
        .into_query_update_uniform(offset_of!(UnlitMaterialUniform, color), cx);

      let alpha = global_watch()
        .watch::<AlphaOf<UnlitMaterialAlphaConfig>>()
        .into_query_update_uniform(offset_of!(UnlitMaterialUniform, alpha), cx);

      let alpha_cutoff = global_watch()
        .watch::<AlphaCutoffOf<UnlitMaterialAlphaConfig>>()
        .into_query_update_uniform(offset_of!(UnlitMaterialUniform, alpha_cutoff), cx);

      source
        .with_source(color)
        .with_source(alpha)
        .with_source(alpha_cutoff)
    },
  );

  let tex_uniform = cx
    .use_uniform_buffers::<EntityHandle<UnlitMaterialEntity>, UnlitMaterialTextureHandlesUniform>(
      |source, cx| {
        let color_alpha_texture =
          offset_of!(UnlitMaterialTextureHandlesUniform, color_alpha_texture);
        add_tex_watcher::<UnlitMaterialColorAlphaTex, _>(source, color_alpha_texture, cx)
      },
    );

  cx.when_create_impl(|| UnlitMaterialDefaultRenderImpl {
    material_access: global_entity_component_of::<StandardModelRefUnlitMaterial>()
      .read_foreign_key(),
    uniforms: uniform.unwrap(),
    texture_uniforms: tex_uniform.unwrap(),
    color_tex_sampler: TextureSamplerIdView::read_from_global(),
    alpha_mode: global_entity_component_of().read(),
  })
}

pub struct UnlitMaterialDefaultRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefUnlitMaterial>,
  uniforms: LockReadGuardHolder<UnlitMaterialUniforms>,
  texture_uniforms: LockReadGuardHolder<UnlitMaterialTexUniforms>,
  color_tex_sampler: TextureSamplerIdView<UnlitMaterialColorAlphaTex>,
  alpha_mode: ComponentReadView<AlphaModeOf<UnlitMaterialAlphaConfig>>,
}

impl GLESModelMaterialRenderImpl for UnlitMaterialDefaultRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    Some(Box::new(UnlitMaterialGPU {
      uniform: self.uniforms.get(&idx)?,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      binding_sys: cx,
      tex_uniform: self.texture_uniforms.get(&idx)?,
      color_alpha_tex_sampler: self.color_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
    }))
  }
}

#[derive(Default)]
pub struct PbrMRMaterialDefaultRenderImplProvider {
  uniforms: QueryToken,
  tex_uniforms: QueryToken,
}

impl QueryBasedFeature<Box<dyn GLESModelMaterialRenderImpl>>
  for PbrMRMaterialDefaultRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, gpu: &GPU) {
    self.uniforms = qcx.register_multi_updater(pbr_mr_material_uniforms(gpu));
    self.tex_uniforms = qcx.register_multi_updater(pbr_mr_material_tex_uniforms(gpu));
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.uniforms);
    qcx.deregister(&mut self.tex_uniforms);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(PbrMRMaterialDefaultRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrMRMaterial>()
        .read_foreign_key(),
      uniforms: cx.take_multi_updater_updated(self.uniforms).unwrap(),
      tex_uniforms: cx.take_multi_updater_updated(self.tex_uniforms).unwrap(),
      alpha_mode: global_entity_component_of().read(),
      base_color_tex_sampler: TextureSamplerIdView::read_from_global(),
      mr_tex_sampler: TextureSamplerIdView::read_from_global(),
      emissive_tex_sampler: TextureSamplerIdView::read_from_global(),
      normal_tex_sampler: TextureSamplerIdView::read_from_global(),
    })
  }
}

struct PbrMRMaterialDefaultRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefPbrMRMaterial>,
  uniforms: LockReadGuardHolder<PbrMRMaterialUniforms>,
  tex_uniforms: LockReadGuardHolder<PbrMRMaterialTexUniforms>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrMRMaterialAlphaConfig>>,
  base_color_tex_sampler: TextureSamplerIdView<PbrMRMaterialBaseColorAlphaTex>,
  mr_tex_sampler: TextureSamplerIdView<PbrMRMaterialMetallicRoughnessTex>,
  emissive_tex_sampler: TextureSamplerIdView<PbrMRMaterialEmissiveTex>,
  normal_tex_sampler: TextureSamplerIdView<NormalTexSamplerOf<PbrMRMaterialNormalInfo>>,
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

impl GLESModelMaterialRenderImpl for PbrMRMaterialDefaultRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    let r = PhysicalMetallicRoughnessMaterialGPU {
      uniform: self.uniforms.get(&idx)?,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      base_color_alpha_tex_sampler: self.base_color_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      mr_tex_sampler: self.mr_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      emissive_tex_sampler: self.emissive_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      normal_tex_sampler: self.normal_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      texture_uniforms: self.tex_uniforms.get(&idx)?,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
}

#[derive(Default)]
pub struct PbrSGMaterialDefaultRenderImplProvider {
  uniforms: QueryToken,
  tex_uniforms: QueryToken,
}

impl QueryBasedFeature<Box<dyn GLESModelMaterialRenderImpl>>
  for PbrSGMaterialDefaultRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.uniforms = qcx.register_multi_updater(pbr_sg_material_uniforms(cx));
    self.tex_uniforms = qcx.register_multi_updater(pbr_sg_material_tex_uniforms(cx));
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.uniforms);
    qcx.deregister(&mut self.tex_uniforms);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(PbrSGMaterialDefaultRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefPbrSGMaterial>()
        .read_foreign_key(),
      uniforms: cx.take_multi_updater_updated(self.uniforms).unwrap(),
      tex_uniforms: cx.take_multi_updater_updated(self.tex_uniforms).unwrap(),
      alpha_mode: global_entity_component_of().read(),
      albedo_tex_sampler: TextureSamplerIdView::read_from_global(),
      specular_glossiness_tex_sampler: TextureSamplerIdView::read_from_global(),
      emissive_tex_sampler: TextureSamplerIdView::read_from_global(),
      normal_tex_sampler: TextureSamplerIdView::read_from_global(),
    })
  }
}

struct PbrSGMaterialDefaultRenderImpl {
  material_access: ForeignKeyReadView<StandardModelRefPbrSGMaterial>,
  uniforms: LockReadGuardHolder<PbrSGMaterialUniforms>,
  tex_uniforms: LockReadGuardHolder<PbrSGMaterialTexUniforms>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrSGMaterialAlphaConfig>>,
  albedo_tex_sampler: TextureSamplerIdView<PbrSGMaterialAlbedoAlphaTex>,
  specular_glossiness_tex_sampler: TextureSamplerIdView<PbrSGMaterialSpecularGlossinessTex>,
  emissive_tex_sampler: TextureSamplerIdView<PbrSGMaterialEmissiveTex>,
  normal_tex_sampler: TextureSamplerIdView<NormalTexSamplerOf<PbrSGMaterialNormalInfo>>,
}

impl GLESModelMaterialRenderImpl for PbrSGMaterialDefaultRenderImpl {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(idx)?;
    let r = PhysicalSpecularGlossinessMaterialGPU {
      uniform: self.uniforms.get(&idx)?,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      albedo_alpha_tex_sampler: self.albedo_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      specular_glossiness_tex_sampler: self
        .specular_glossiness_tex_sampler
        .get_pair(idx)
        .unwrap_or(EMPTY_H),
      emissive_tex_sampler: self.emissive_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      normal_tex_sampler: self.normal_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
      texture_uniforms: self.tex_uniforms.get(&idx)?,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
}
