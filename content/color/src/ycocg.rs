use crate::LinearRGBColor;

/// https://en.wikipedia.org/wiki/YCoCg
#[repr(C)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct YCoCgRColor<T> {
  pub y: T,
  pub co: T,
  pub cg: T,
}

unsafe impl<T: bytemuck::Pod> bytemuck::Pod for YCoCgRColor<T> {}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for YCoCgRColor<T> {}

impl From<YCoCgRColor<f32>> for LinearRGBColor<f32> {
  fn from(color: YCoCgRColor<f32>) -> Self {
    let tmp = color.y - color.cg * 0.5;
    let g = color.cg + tmp;
    let b = tmp - color.co * 0.5;
    let r = b + color.co;

    Self { r, g, b }
  }
}

impl From<LinearRGBColor<f32>> for YCoCgRColor<f32> {
  fn from(color: LinearRGBColor<f32>) -> Self {
    let co = color.r - color.b;
    let tmp = color.b + co * 0.5;
    let cg = color.g - tmp;
    let y = tmp + cg * 0.5;

    Self { y, co, cg }
  }
}
