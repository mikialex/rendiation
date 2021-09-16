use std::{
  num::NonZeroUsize,
  ops::{Deref, DerefMut},
};

use image::*;
use rendiation_texture_types::Size;
use rendiation_webgpu::{TextureFormat, WebGPUTexture2dSource};

use crate::Texture2DSource;

pub trait TextureFormatDecider {
  const FORMAT: TextureFormat;
}
impl TextureFormatDecider for image::Rgba<u8> {
  const FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
}

// https://github.com/gpuweb/gpuweb/issues/66
pub fn rgb_to_rgba(input: ImageBuffer<Rgb<u8>, Vec<u8>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
  let mut target = ImageBuffer::new(input.width(), input.height());
  // todo: could be optimized
  target
    .pixels_mut()
    .zip(input.pixels())
    .for_each(|(target, source)| {
      *target = Rgba([source.0[0], source.0[1], source.0[2], 255]);
    });
  target
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
