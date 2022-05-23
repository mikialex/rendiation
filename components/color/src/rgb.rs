use rendiation_algebra::{Scalar, Vec3};

use crate::RGBColor;

#[repr(C)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct LinearRGBColor<T> {
  pub r: T,
  pub g: T,
  pub b: T,
}

unsafe impl<T: bytemuck::Pod> bytemuck::Pod for LinearRGBColor<T> {}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for LinearRGBColor<T> {}

impl<T: Scalar> RGBColor<T> for LinearRGBColor<T> {}
impl<T: Scalar> std::ops::Mul<T> for LinearRGBColor<T> {
  type Output = Self;

  fn mul(self, rhs: T) -> Self::Output {
    Self {
      r: self.r * rhs,
      g: self.g * rhs,
      b: self.b * rhs,
    }
  }
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

impl<T> From<(T, T, T)> for LinearRGBColor<T> {
  fn from(value: (T, T, T)) -> Self {
    Self {
      r: value.0,
      g: value.1,
      b: value.2,
    }
  }
}

impl<T> From<LinearRGBColor<T>> for (T, T, T) {
  fn from(value: LinearRGBColor<T>) -> Self {
    (value.r, value.g, value.b)
  }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SRGBColor<T> {
  pub r: T,
  pub g: T,
  pub b: T,
}

unsafe impl<T: bytemuck::Pod> bytemuck::Pod for SRGBColor<T> {}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for SRGBColor<T> {}

impl<T: Scalar> RGBColor<T> for SRGBColor<T> {}
impl<T: Scalar> std::ops::Mul<T> for SRGBColor<T> {
  type Output = Self;

  fn mul(self, rhs: T) -> Self::Output {
    Self {
      r: self.r * rhs,
      g: self.g * rhs,
      b: self.b * rhs,
    }
  }
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

impl<T> From<Vec3<T>> for SRGBColor<T> {
  fn from(value: Vec3<T>) -> Self {
    Self {
      r: value.x,
      g: value.y,
      b: value.z,
    }
  }
}

impl<T> From<SRGBColor<T>> for Vec3<T> {
  fn from(value: SRGBColor<T>) -> Self {
    Vec3::new(value.r, value.g, value.b)
  }
}

impl<T> From<(T, T, T)> for SRGBColor<T> {
  fn from(value: (T, T, T)) -> Self {
    Self {
      r: value.0,
      g: value.1,
      b: value.2,
    }
  }
}

impl<T> From<SRGBColor<T>> for (T, T, T) {
  fn from(value: SRGBColor<T>) -> Self {
    (value.r, value.g, value.b)
  }
}
