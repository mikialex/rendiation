use crate::*;

impl BindableResourceView for gpu::TlasPackage {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::AccelerationStructure(self.tlas())
  }
}
impl BindableResourceView for gpu::Tlas {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::AccelerationStructure(self)
  }
}

pub type GPUTlas = ResourceRc<gpu::TlasPackage>;
pub type GPUTlasView = ResourceViewRc<gpu::TlasPackage>;

#[derive(Clone)]
pub struct GPUTlasSource {
  pub instances: Vec<Option<TlasInstance>>,
  pub flags: wgpu_types::AccelerationStructureFlags,
  pub update_mode: wgpu_types::AccelerationStructureUpdateMode,
}

impl Resource for gpu::TlasPackage {
  type Descriptor = GPUTlasSource;

  type View = gpu::Tlas;

  type ViewDescriptor = ();

  fn create_view(&self, _desc: &Self::ViewDescriptor) -> Self::View {
    self.tlas().clone()
  }
}

impl BindableResourceProvider for GPUTlasView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::AccelerationStructure(self.clone())
  }
}

impl InitResourceByAllocation for gpu::TlasPackage {
  fn create_resource(tlas_source: &Self::Descriptor, device: &GPUDevice) -> Self {
    let tlas = device.create_tlas(&CreateTlasDescriptor {
      label: None,
      max_instances: tlas_source.instances.len() as u32,
      flags: tlas_source.flags,
      update_mode: tlas_source.update_mode,
    });
    Self::new_with_instances(tlas, tlas_source.instances.clone())
  }
}
