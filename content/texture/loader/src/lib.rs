use std::{
  num::NonZeroUsize,
  ops::{Deref, DerefMut},
  path::Path,
};

use image::*;
use rendiation_algebra::*;
use rendiation_texture_core::*;

pub struct ImageLibContainerWrap<T>(pub T);

impl<P, C> Texture2D for ImageLibContainerWrap<ImageBuffer<P, C>>
where
  P: Pixel + 'static,
  [P::Subpixel]: EncodableLayout,
  C: Deref<Target = [P::Subpixel]>,
  C: DerefMut<Target = [P::Subpixel]>,
{
  type Pixel = P;

  fn get(&self, position: impl Into<Vec2<usize>>) -> &Self::Pixel {
    let position = position.into();
    self.0.get_pixel(position.x as u32, position.y as u32)
  }

  fn get_mut(&mut self, position: impl Into<Vec2<usize>>) -> &mut Self::Pixel {
    let position = position.into();
    self.0.get_pixel_mut(position.x as u32, position.y as u32)
  }

  fn size(&self) -> Size {
    let d = self.0.dimensions();
    Size {
      width: NonZeroUsize::new(d.0 as usize).unwrap(),
      height: NonZeroUsize::new(d.1 as usize).unwrap(),
    }
  }
}

impl Texture2dInitAble for ImageLibContainerWrap<ImageBuffer<Rgba<u8>, Vec<u8>>> {
  fn init_with(size: Size, pixel: Self::Pixel) -> Self {
    let mut result = ImageLibContainerWrap(ImageBuffer::new(
      <usize as std::convert::From<_>>::from(size.width) as u32,
      <usize as std::convert::From<_>>::from(size.height) as u32,
    ));
    result.clear(pixel);
    result
  }

  #[allow(clippy::uninit_vec)]
  fn init_not_care(size: Size) -> Self {
    let width = <usize as std::convert::From<_>>::from(size.width);
    let height = <usize as std::convert::From<_>>::from(size.height);
    let mut buffer = Vec::with_capacity(width * height * 4);
    unsafe { buffer.set_len(width * height * 4) };
    ImageLibContainerWrap(ImageBuffer::from_raw(width as u32, height as u32, buffer).unwrap())
  }
}

// todo texture loader should passed in and config ability freely
pub fn load_tex(path: impl AsRef<Path>) -> GPUBufferImage {
  use image::io::Reader as ImageReader;
  let img = ImageReader::open(path).unwrap().decode().unwrap();
  match img {
    image::DynamicImage::ImageRgba8(img) => {
      let img = ImageLibContainerWrap(img);
      let size = img.size();
      let format = TextureFormat::Rgba8UnormSrgb;
      let data = img.0.into_raw();
      GPUBufferImage { data, format, size }
    }
    image::DynamicImage::ImageRgb8(img) => {
      let img = ImageLibContainerWrap(img);
      let size = img.size();
      let format = TextureFormat::Rgba8UnormSrgb;
      let data = create_padding_buffer(img.0.as_raw(), 3, &[255]);
      GPUBufferImage { data, format, size }
    }
    _ => panic!("unsupported texture type"),
  }
}
