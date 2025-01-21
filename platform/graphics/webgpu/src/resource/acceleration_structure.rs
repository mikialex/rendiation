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

impl Resource for gpu::TlasPackage {
  type Descriptor = gpu::CreateTlasDescriptor<'static>;

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
  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    let tlas = device.create_tlas(desc);
    Self::new(tlas)
  }
}
