pub mod d2;
pub use d2::*;
pub mod cube;
pub use cube::*;

use crate::*;

pub type GPUTexture = ResourceRc<gpu::Texture>;
pub type GPUTextureView = ResourceViewRc<gpu::Texture>;

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
    todo!()
  }
}

#[derive(Clone)]
pub struct GPU2DTexture(pub GPUTexture);

#[derive(Clone)]
pub struct GPUCubeTexture(pub GPUTexture);

#[derive(Clone)]
pub struct GPU1DTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU1DArrayTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU2DTextureView(pub GPUTextureView);

impl Deref for GPU2DTextureView {
  type Target = GPUTextureView;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[derive(Clone)]
pub struct GPU2DArrayTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPUCubeTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPUCubeArrayTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU3DTextureView(pub GPUTextureView);
