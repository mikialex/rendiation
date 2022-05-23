use rendiation_algebra::vector;

use crate::Texture2D;

pub struct TexturePixels<'a, T> {
  pub(crate) texture: &'a T,
  pub(crate) current: usize,
  pub(crate) all: usize,
}

impl<'a, T: Texture2D> Iterator for TexturePixels<'a, T> {
  type Item = (&'a T::Pixel, (usize, usize));

  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.all {
      return None;
    }
    let width = self.texture.size().width;
    let x = self.current % width;
    let y = self.current / width;
    self.current += 1;
    Some((self.texture.get(vector!(x, y)), (x, y)))
  }
}

pub struct TexturePixelsMut<'a, T> {
  pub(crate) texture: &'a mut T,
  pub(crate) current: usize,
  pub(crate) all: usize,
}

impl<'a, T: Texture2D> Iterator for TexturePixelsMut<'a, T> {
  type Item = (&'a mut T::Pixel, (usize, usize));

  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.all {
      return None;
    }
    let width = self.texture.size().width;
    let x = self.current % width;
    let y = self.current / width;
    self.current += 1;
    let pixel = unsafe { std::mem::transmute(self.texture.get_mut(vector!(x, y))) };
    Some((pixel, (x, y)))
  }
}
