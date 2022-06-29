pub mod d1;
pub use d1::*;
pub mod d2;
pub use d2::*;
pub mod d3;
pub use d3::*;
pub mod cube;
pub use cube::*;

use crate::*;

pub type GPUTexture = ResourceRc<gpu::Texture>;

impl Resource for gpu::Texture {
  type Descriptor = gpu::TextureDescriptor<'static>;

  type View = gpu::TextureView;

  type ViewDescriptor = gpu::TextureViewDescriptor<'static>;

  fn create_view(&self, desc: &Self::ViewDescriptor) -> Self::View {
    self.create_view(desc)
  }
}

impl InitResourceByAllocation for gpu::Texture {
  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    device.create_texture(desc)
  }
}

impl BindableResourceView for gpu::TextureView {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::TextureView(self)
  }
}
