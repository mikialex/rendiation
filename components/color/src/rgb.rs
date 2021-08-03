use rendiation_algebra::Vec3;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct LinearRGBColor<T> {
  pub r: T,
  pub g: T,
  pub b: T,
}

impl<T> LinearRGBColor<T> {
  pub fn new(r: T, g: T, b: T) -> Self {
    Self { r, g, b }
  }
}

impl<T: Copy> LinearRGBColor<T> {
  pub fn splat(v: T) -> Self {
    Self { r: v, g: v, b: v }
  }
}

impl<T> From<Vec3<T>> for LinearRGBColor<T> {
  fn from(value: Vec3<T>) -> Self {
    Self {
      r: value.x,
      g: value.y,
      b: value.z,
    }
  }
}

impl<T> From<LinearRGBColor<T>> for Vec3<T> {
  fn from(value: LinearRGBColor<T>) -> Self {
    Vec3::new(value.r, value.g, value.b)
  }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SRGBColor<T> {
  pub r: T,
  pub g: T,
  pub b: T,
}

#[allow(clippy::excessive_precision)]
impl From<SRGBColor<f32>> for LinearRGBColor<f32> {
  fn from(color: SRGBColor<f32>) -> Self {
    fn convert(c: f32) -> f32 {
      if c < 0.04045 {
        c * 0.0773993808
      } else {
        (c * 0.9478672986 + 0.0521327014).powf(2.4)
      }
    }
    Self {
      r: convert(color.r),
      g: convert(color.g),
      b: convert(color.b),
    }
  }
}

impl From<LinearRGBColor<f32>> for SRGBColor<f32> {
  fn from(color: LinearRGBColor<f32>) -> Self {
    fn convert(c: f32) -> f32 {
      if c < 0.0031308 {
        c * 12.92
      } else {
        1.055 * (c.powf(0.41666)) - 0.055
      }
    }
    Self {
      r: convert(color.r),
      g: convert(color.g),
      b: convert(color.b),
    }
  }
}
