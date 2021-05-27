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
    Some((self.texture.get(vector!(x, y)), (x, y)))
  }
}
