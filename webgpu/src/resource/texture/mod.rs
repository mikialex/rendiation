pub mod d2;
pub use d2::*;
pub mod cube;
pub use cube::*;

use crate::*;

pub type GPUTexture = ResourceRc<wgpu::Texture>;

impl Resource for wgpu::Texture {
  type Descriptor = wgpu::TextureDescriptor<'static>;

  type View = wgpu::TextureView;

  type ViewDescriptor = wgpu::TextureViewDescriptor<'static>;

  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    device.create_texture(desc)
  }

  fn create_view(&self, desc: &Self::ViewDescriptor) -> Self::View {
    self.create_view(desc)
  }
}

impl BindableResourceView for wgpu::TextureView {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(self)
  }
}
