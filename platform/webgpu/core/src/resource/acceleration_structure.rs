use crate::*;

// todo, support partial updates(supported in wgpu)
#[derive(Clone)]
pub struct GPUTlasRaw {
  internal: Arc<gpu::Tlas>,
}

impl GPUTlasRaw {
  pub fn gpu(&self) -> &gpu::Tlas {
    &self.internal
  }
}

impl BindableResourceView for GPUTlasRaw {
  fn as_bindable(&self) -> gpu::BindingResource<'_> {
    gpu::BindingResource::AccelerationStructure(&self.internal)
  }
}

pub type GPUTlas = ResourceRc<GPUTlasRaw>;
pub type GPUTlasView = ResourceViewRc<GPUTlasRaw>;

#[derive(Clone)]
pub struct GPUTlasSource {
  pub instances: Vec<Option<TlasInstance>>,
  pub flags: wgpu_types::AccelerationStructureFlags,
  pub update_mode: wgpu_types::AccelerationStructureUpdateMode,
}

impl Resource for GPUTlasRaw {
  type Descriptor = GPUTlasSource;

  type View = GPUTlasRaw;

  type ViewDescriptor = ();

  fn create_view(&self, _desc: &Self::ViewDescriptor) -> Self::View {
    self.clone()
  }
}

impl BindableResourceProvider for GPUTlasView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::AccelerationStructure(self.view.clone())
  }
}

impl InitResourceByAllocation for GPUTlasRaw {
  fn create_resource(tlas_source: &Self::Descriptor, device: &GPUDevice) -> Self {
    let mut tlas = device.create_tlas(&CreateTlasDescriptor {
      label: None,
      max_instances: tlas_source.instances.len() as u32,
      flags: tlas_source.flags,
      update_mode: tlas_source.update_mode,
    });
    let instances = tlas.get_mut_slice(0..tlas_source.instances.len()).unwrap();
    instances.clone_from_slice(&tlas_source.instances);

    GPUTlasRaw {
      internal: Arc::new(tlas),
    }
  }
}
