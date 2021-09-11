use std::{
  num::NonZeroUsize,
  ops::{Deref, DerefMut},
};

use image::EncodableLayout;
use rendiation_texture_types::Size;
use rendiation_webgpu::{TextureFormat, WebGPUTexture2dSource};

use crate::Texture2DSource;

pub trait TextureFormatDecider {
  const FORMAT: TextureFormat;
}
impl TextureFormatDecider for image::Rgba<u8> {
  const FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
}

impl<P, C> WebGPUTexture2dSource for Texture2DSource<image::ImageBuffer<P, C>>
where
  P: TextureFormatDecider + image::Pixel + 'static,
  [P::Subpixel]: EncodableLayout,
  C: Deref<Target = [P::Subpixel]>,
  C: DerefMut<Target = [P::Subpixel]>,
  C: AsRef<[u8]>,
{
  fn format(&self) -> TextureFormat {
    P::FORMAT
  }

  fn bytes_per_pixel(&self) -> usize {
    return 4;
  }

  fn as_bytes(&self) -> &[u8] {
    self.as_raw().as_ref()
  }

  fn size(&self) -> Size {
    Size {
      width: NonZeroUsize::new(self.width() as usize).unwrap(),
      height: NonZeroUsize::new(self.height() as usize).unwrap(),
    }
  }
}
