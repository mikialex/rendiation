use std::{
  cell::RefCell,
  rc::{Rc, Weak},
};

use super::{
  MaterialHandle, SamplerHandle, SceneMaterialRenderPrepareCtx, SceneSampler, SceneTexture2D,
  Texture2DHandle, WatchedArena,
};

pub struct BindGroup {
  pub gpu: wgpu::BindGroup,
  references: Vec<MaterialTextureReferenceFinalizer>,
}

#[derive(Default)]
pub struct ReferenceFinalization {
  deleting: Rc<RefCell<Vec<ReferenceRecord>>>,
}

impl ReferenceFinalization {
  pub fn maintain(
    &mut self,
    samplers: &WatchedArena<SceneSampler>,
    texture_2ds: &WatchedArena<SceneTexture2D>,
  ) {
    self.deleting.borrow_mut().drain(..).for_each(|r| {
      let material = r.material;
      match r.resource {
        ResourceReference::Texture2d(tex) => texture_2ds
          .get_resource(tex)
          .unwrap()
          .remove_material_bind(material),
        ResourceReference::Sampler(s) => samplers
          .get_resource(s)
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
  Sampler(SamplerHandle),
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

  pub fn push_sampler<'c: 'b, 'd: 'b>(
    mut self,
    ctx: &'c SceneMaterialRenderPrepareCtx<'d>,
    handle: SamplerHandle,
  ) -> Self {
    self.bindings.push(
      ctx
        .samplers
        .get_resource(handle)
        .unwrap()
        .as_material_bind(self.handle),
    );

    self.references.push(MaterialTextureReferenceFinalizer {
      reference: ReferenceRecord {
        material: self.handle,
        resource: ResourceReference::Sampler(handle),
      },
      sender: ctx.reference_finalization.create_sender(),
    });
    self
  }

  pub fn build(self, layout: &wgpu::BindGroupLayout) -> BindGroup {
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

    BindGroup {
      gpu,
      references: self.references,
    }
  }
}
