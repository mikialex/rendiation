pub mod d2;
pub use d2::*;
pub mod cube;
pub use cube::*;

pub struct WebGPUTexture {
  pub texture: wgpu::Texture,
  pub desc: wgpu::TextureDescriptor<'static>,
}

impl std::ops::Deref for WebGPUTexture {
  type Target = wgpu::Texture;

  fn deref(&self) -> &Self::Target {
    &self.texture
  }
}

// pub struct Tex<
//   const DIMENSION: wgpu::TextureDimension,
//   const FORMAT: usize,
//   const MULTI_SAMPLE: bool,
// > {
//   pub texture: wgpu::Texture,
//   pub desc: wgpu::TextureDescriptor<'static>,
// }
