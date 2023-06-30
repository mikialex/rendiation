use std::{
  io::Error,
  ops::{Deref, DerefMut},
  path::Path,
};

pub trait TextureIO<T> {
  fn save_to_file(&self, path: &dyn AsRef<Path>) -> Result<(), Error>;
}

pub struct PNG;

use image::{EncodableLayout, ImageBuffer, Pixel, PixelWithColorType};
impl<P, C> TextureIO<PNG> for ImageBuffer<P, C>
where
  P: Pixel + PixelWithColorType + 'static,
  [P::Subpixel]: EncodableLayout,
  C: Deref<Target = [P::Subpixel]>,
  C: DerefMut<Target = [P::Subpixel]>,
{
  fn save_to_file(&self, path: &dyn AsRef<Path>) -> Result<(), Error> {
    self
      .save_with_format(path, image::ImageFormat::Png)
      .map_err(|e| match e {
        image::ImageError::IoError(io) => io,
        _ => unreachable!(),
      })
  }
}
