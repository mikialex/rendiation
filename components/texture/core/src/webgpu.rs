use std::{
  num::NonZeroUsize,
  ops::{Deref, DerefMut},
};

use image::*;
use rendiation_texture_types::Size;
use rendiation_webgpu::{TextureFormat, WebGPUTexture2dSource};

use crate::{Texture2D, Texture2DBuffer, Texture2DSource};

pub trait TextureFormatDecider {
  const FORMAT: TextureFormat;
}
impl TextureFormatDecider for image::Rgba<u8> {
  const FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
}
// todo how do we support int texture?? by adding new type?
impl TextureFormatDecider for u8 {
  const FORMAT: TextureFormat = TextureFormat::R8Unorm;
}

// https://github.com/gpuweb/gpuweb/issues/66
pub fn rgb_to_rgba(input: ImageBuffer<Rgb<u8>, Vec<u8>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
  input.map(|r| Rgba([r[0], r[1], r[2], 255]))
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

impl<P> WebGPUTexture2dSource for Texture2DBuffer<P>
where
  P: TextureFormatDecider + Clone,
{
  fn format(&self) -> TextureFormat {
    P::FORMAT
  }

  fn as_bytes(&self) -> &[u8] {
    self.as_byte_buffer()
  }

  fn size(&self) -> Size {
    self.size()
  }
}
