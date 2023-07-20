use std::{
  io::Error,
  ops::{Deref, DerefMut},
  path::Path,
};

pub trait TextureIO<T> {
  fn save_to_file(&self, path: &dyn AsRef<Path>) -> Result<(), Error>;
}

pub struct PNG;

use fast_hash_collection::FastHashMap;
use image::{EncodableLayout, ImageBuffer, Pixel, PixelWithColorType};
use rendiation_texture_types::Size;
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

pub trait AbstractTextureLoader<P> {
  fn load(&self, on_pixel: &mut dyn FnMut()) -> Result<Size, Error>;
}

/// When user want to load arbitrary image files in different formats, it's
/// convenient to use the so called dynamic loader which contains file format support
/// as much as possible. However which specific formats a dynamic loader should support
/// could or should be decided in runtime to avoid unnecessary binary bloat.
pub struct DynamicTextureLoader<P> {
  supports: FastHashMap<&'static str, Box<dyn AbstractTextureLoader<P>>>,
}

impl<P> DynamicTextureLoader<P> {
  pub fn register_loader(
    &mut self,
    ext_name: &'static str,
    loader: impl AbstractTextureLoader<P> + 'static,
  ) -> &mut Self {
    self.supports.insert(ext_name, Box::new(loader));
    self
  }
}
