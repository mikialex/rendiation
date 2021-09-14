use std::{
  cell::RefCell,
  rc::{Rc, Weak},
};

use rendiation_texture::{AddressMode, FilterMode, TextureSampler};

use crate::{SceneTextureCube, TextureCubeHandle};

use super::{
  MaterialHandle, SceneMaterialRenderPrepareCtx, SceneTexture2D, Texture2DHandle, WatchedArena,
};

pub struct MaterialBindGroup {
  pub gpu: wgpu::BindGroup,
  _references: Vec<MaterialTextureReferenceFinalizer>,
}

#[derive(Default)]
pub struct ReferenceFinalization {
  deleting: Rc<RefCell<Vec<ReferenceRecord>>>,
}

impl ReferenceFinalization {
  pub fn maintain(
    &mut self,
    texture_2ds: &WatchedArena<SceneTexture2D>,
    texture_cubes: &WatchedArena<SceneTextureCube>,
  ) {
    self.deleting.borrow_mut().drain(..).for_each(|r| {
      let material = r.material;
      match r.resource {
        ResourceReference::Texture2d(tex) => texture_2ds
          .get_resource(tex)
          .unwrap()
          .remove_material_bind(material),
        ResourceReference::TextureCube(tex) => texture_cubes
          .get_resource(tex)
          .unwrap()
          .remove_material_bind(material),
      }
    })
  }

  pub fn create_sender(&self) -> ReferenceSender {
    ReferenceSender {
      deleting: Rc::downgrade(&self.deleting),
    }
  }
}

pub struct ReferenceSender {
  deleting: Weak<RefCell<Vec<ReferenceRecord>>>,
}

#[derive(Clone, Copy)]
pub struct ReferenceRecord {
  material: MaterialHandle,
  resource: ResourceReference,
}

#[derive(Clone, Copy)]
pub enum ResourceReference {
  Texture2d(Texture2DHandle),
  TextureCube(TextureCubeHandle),
}

pub struct MaterialTextureReferenceFinalizer {
  reference: ReferenceRecord,
  sender: ReferenceSender,
}

impl Drop for MaterialTextureReferenceFinalizer {
  fn drop(&mut self) {
    self.sender.deleting.upgrade().map(|deleting| {
      deleting
        .try_borrow_mut()
        .map(|mut deleting| deleting.push(self.reference))
    });
  }
}

pub struct MaterialBindGroupBuilder<'a, 'b> {
  handle: MaterialHandle,
  device: &'a wgpu::Device,
  bindings: Vec<wgpu::BindingResource<'b>>,
  references: Vec<MaterialTextureReferenceFinalizer>,
}

pub trait ViewerDeviceExt {
  fn material_bindgroup_builder<'a, 'b>(
    &'a self,
    handle: MaterialHandle,
  ) -> MaterialBindGroupBuilder<'a, 'b>;
}
impl ViewerDeviceExt for wgpu::Device {
  fn material_bindgroup_builder<'a, 'b>(
    &'a self,
    handle: MaterialHandle,
  ) -> MaterialBindGroupBuilder<'a, 'b> {
    MaterialBindGroupBuilder {
      handle,
      device: self,
      bindings: Vec::with_capacity(4),
      references: Vec::with_capacity(4),
    }
  }
}

impl<'a> SceneMaterialRenderPrepareCtx<'a> {
  pub fn map_sampler(&mut self, sampler: TextureSampler) -> Rc<wgpu::Sampler> {
    fn convert_wrap(mode: AddressMode) -> wgpu::AddressMode {
      match mode {
        AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        AddressMode::Repeat => wgpu::AddressMode::Repeat,
        AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        AddressMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
      }
    }
    fn convert_filter(mode: FilterMode) -> wgpu::FilterMode {
      match mode {
        FilterMode::Nearest => wgpu::FilterMode::Nearest,
        FilterMode::Linear => wgpu::FilterMode::Linear,
      }
    }

    fn convert(sampler: TextureSampler) -> wgpu::SamplerDescriptor<'static> {
      wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: convert_wrap(sampler.address_mode_u),
        address_mode_v: convert_wrap(sampler.address_mode_v),
        address_mode_w: convert_wrap(sampler.address_mode_w),
        mag_filter: convert_filter(sampler.mag_filter),
        min_filter: convert_filter(sampler.min_filter),
        mipmap_filter: convert_filter(sampler.mipmap_filter),
        ..Default::default()
      }
    }

    self
      .samplers
      .entry(sampler)
      .or_insert_with(|| Rc::new(self.gpu.device.create_sampler(&convert(sampler))))
      .clone()
  }
}

impl<'a, 'b> MaterialBindGroupBuilder<'a, 'b> {
  pub fn push(mut self, binding: wgpu::BindingResource<'b>) -> Self {
    self.bindings.push(binding);
    self
  }

  pub fn push_texture2d<'c: 'b, 'd: 'b>(
    mut self,
    ctx: &'c SceneMaterialRenderPrepareCtx<'d>,
    handle: Texture2DHandle,
  ) -> Self {
    self.bindings.push(
      ctx
        .textures
        .get_resource(handle)
        .unwrap()
        .as_material_bind(self.handle),
    );

    self.references.push(MaterialTextureReferenceFinalizer {
      reference: ReferenceRecord {
        material: self.handle,
        resource: ResourceReference::Texture2d(handle),
      },
      sender: ctx.reference_finalization.create_sender(),
    });
    self
  }

  pub fn push_texture_cube<'c: 'b, 'd: 'b>(
    mut self,
    ctx: &'c SceneMaterialRenderPrepareCtx<'d>,
    handle: TextureCubeHandle,
  ) -> Self {
    self.bindings.push(
      ctx
        .texture_cubes
        .get_resource(handle)
        .unwrap()
        .as_material_bind(self.handle),
    );

    self.references.push(MaterialTextureReferenceFinalizer {
      reference: ReferenceRecord {
        material: self.handle,
        resource: ResourceReference::TextureCube(handle),
      },
      sender: ctx.reference_finalization.create_sender(),
    });
    self
  }

  pub fn build(self, layout: &wgpu::BindGroupLayout) -> MaterialBindGroup {
    let entries: Vec<_> = self
      .bindings
      .into_iter()
      .enumerate()
      .map(|(i, resource)| wgpu::BindGroupEntry {
        binding: i as u32,
        resource,
      })
      .collect();

    let gpu = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &entries,
      label: None,
    });

    MaterialBindGroup {
      gpu,
      _references: self.references,
    }
  }
}
