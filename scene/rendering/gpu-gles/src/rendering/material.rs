use rendiation_texture_gpu_system::GPUTextureBindingSystem;

use crate::*;

pub trait GLESModelMaterialRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>>;
}

impl GLESModelMaterialRenderImpl for Vec<Box<dyn GLESModelMaterialRenderImpl>> {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>> {
    for provider in self {
      if let Some(com) = provider.make_component(idx) {
        return Some(com);
      }
    }
    None
  }
}

pub struct FlatMaterialDefaultRenderImplProvider;
impl RenderImplProvider<Box<dyn GLESModelMaterialRenderImpl>>
  for FlatMaterialDefaultRenderImplProvider
{
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    let updater = flat_material_uniforms(res.cx());
    res.register_multi_updater(updater);
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(FlatMaterialDefaultRenderImpl {
      material_access: global_entity_component_of::<StandardModelRefFlatMaterial>().read(),
      uniforms: res.get_multi_updater().unwrap(),
    })
  }
}

struct FlatMaterialDefaultRenderImpl {
  material_access: ComponentReadView<StandardModelRefFlatMaterial>,
  uniforms: LockReadGuardHolder<FlatMaterialUniforms>,
}

impl GLESModelMaterialRenderImpl for FlatMaterialDefaultRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny + '_>> {
    let idx = self.material_access.get(idx)?;
    let idx = (*idx)?;
    Some(Box::new(FlatMaterialGPU {
      uniform: self.uniforms.get(&idx.into())?,
    }))
  }
}

pub struct PbrMRMaterialDefaultRenderImplProvider;
impl RenderImplProvider<Box<dyn GLESModelMaterialRenderImpl>>
  for PbrMRMaterialDefaultRenderImplProvider
{
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    todo!()
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn GLESModelMaterialRenderImpl> {
    Box::new(PbrMRMaterialDefaultRenderImpl {
      material_access: todo!(),
      uniforms: todo!(),
      tex_uniforms: todo!(),
      alpha_mode: todo!(),
      base_color_tex_sampler: todo!(),
      mr_tex_sampler: todo!(),
      emissive_tex_sampler: todo!(),
      normal_tex_sampler: todo!(),
      binding_sys: todo!(),
    })
  }
}

struct PbrMRMaterialDefaultRenderImpl {
  material_access: ComponentReadView<StandardModelRefPbrMRMaterial>,
  uniforms: PbrMRMaterialUniforms,
  tex_uniforms: PbrMRMaterialTexUniforms,
  alpha_mode: ComponentReadView<PbrMRMaterialAlphaModeComponent>,
  base_color_tex_sampler: ComponentReadView<PbrMRMaterialAlphaModeComponent>,
  mr_tex_sampler: ComponentReadView<PbrMRMaterialAlphaModeComponent>,
  emissive_tex_sampler: ComponentReadView<PbrMRMaterialAlphaModeComponent>,
  normal_tex_sampler: ComponentReadView<PbrMRMaterialAlphaModeComponent>,
  binding_sys: GPUTextureBindingSystem,
}

impl GLESModelMaterialRenderImpl for PbrMRMaterialDefaultRenderImpl {
  fn make_component(
    &self,
    idx: AllocIdx<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponentAny>> {
    let idx = self.material_access.get(idx)?;
    let idx = (*idx)?;
    PhysicalMetallicRoughnessMaterialGPU {
      uniform: self.uniforms.get(&idx.into())?,
      alpha_mode: todo!(),
      base_color_tex_sampler: todo!(),
      mr_tex_sampler: todo!(),
      emissive_tex_sampler: todo!(),
      normal_tex_sampler: todo!(),
      texture_uniforms: self.tex_uniforms.get(&idx.into())?,
      binding_sys: &self.binding_sys,
    };
    None
  }
}
