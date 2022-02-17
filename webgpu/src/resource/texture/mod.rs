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

// pub struct WebGPUTexture {
//   pub texture: wgpu::Texture,
//   pub desc: wgpu::TextureDescriptor<'static>,
// }

// impl std::ops::Deref for WebGPUTexture {
//   type Target = wgpu::Texture;

//   fn deref(&self) -> &Self::Target {
//     &self.texture
//   }
// }

// pub struct Tex<
//   const DIMENSION: wgpu::TextureDimension,
//   const FORMAT: usize,
//   const MULTI_SAMPLE: bool,
// > {
//   pub texture: wgpu::Texture,
//   pub desc: wgpu::TextureDescriptor<'static>,
// }
