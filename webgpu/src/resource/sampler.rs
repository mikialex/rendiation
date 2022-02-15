use crate::*;

impl BindableResourceView for wgpu::Sampler {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::Sampler(self)
  }
}

pub type GPUSampler = ResourceRc<wgpu::Sampler>;

impl Resource for wgpu::Sampler {
  type Descriptor = wgpu::SamplerDescriptor<'static>;

  type View = ();

  type ViewDescriptor = ();

  fn create_resource(desc: &Self::Descriptor, device: &wgpu::Device) -> Self {
    device.create_sampler(desc)
  }

  fn create_view(&self, desc: &Self::ViewDescriptor, device: &wgpu::Device) -> Self::View {}
}
