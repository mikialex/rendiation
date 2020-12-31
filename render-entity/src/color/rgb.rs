use super::{Color, ColorSpace, RGBColorSpace};
use rendiation_math::Vec3;
use std::marker::PhantomData;

pub trait RGBColor<T> {
  fn r(&self) -> T;
  fn g(&self) -> T;
  fn b(&self) -> T;
  fn mut_r(&mut self) -> &mut T;
  fn mut_g(&mut self) -> &mut T;
  fn mut_b(&mut self) -> &mut T;
}

// auto impl <rgb channel fetch> for all color that <marked as rgb colorspace and their value types is vec3<T>>
impl<T: Copy, U: RGBColorSpace<T> + ColorSpace<T, ContainerValue = Vec3<T>>> RGBColor<T>
  for Color<T, U>
{
  fn r(&self) -> T {
    self.value.x
  }
  fn g(&self) -> T {
    self.value.y
  }
  fn b(&self) -> T {
    self.value.z
  }
  fn mut_r(&mut self) -> &mut T {
    &mut self.value.x
  }
  fn mut_g(&mut self) -> &mut T {
    &mut self.value.y
  }
  fn mut_b(&mut self) -> &mut T {
    &mut self.value.z
  }
}

pub struct AnyRGBColorSpace<T: Copy + Clone> {
  phantom: PhantomData<T>,
}
impl<T: Copy + Clone> RGBColorSpace<T> for AnyRGBColorSpace<T> {}
impl<T: Copy + Clone> ColorSpace<T> for AnyRGBColorSpace<T> {
  type ContainerValue = Vec3<T>;
}

pub struct SRGBColorSpace<T: Copy + Clone> {
  phantom: PhantomData<T>,
}
impl<T: Copy + Clone> RGBColorSpace<T> for SRGBColorSpace<T> {}
impl<T: Copy + Clone> ColorSpace<T> for SRGBColorSpace<T> {
  type ContainerValue = Vec3<T>;
}

#[allow(clippy::excessive_precision)]
impl Color<f32, SRGBColorSpace<f32>> {
  pub fn to_linear_rgb(&self) -> Color<f32, LinearRGBColorSpace<f32>> {
    Color::new(self.value.map(|c| {
      if c < 0.04045 {
        c * 0.0773993808
      } else {
        (c * 0.9478672986 + 0.0521327014).powf(2.4)
      }
    }))
  }
}

pub struct LinearRGBColorSpace<T: Copy + Clone> {
  phantom: PhantomData<T>,
}
impl<T: Copy + Clone> RGBColorSpace<T> for LinearRGBColorSpace<T> {}
impl<T: Copy + Clone> ColorSpace<T> for LinearRGBColorSpace<T> {
  type ContainerValue = Vec3<T>;
}

impl Color<f32, LinearRGBColorSpace<f32>> {
  pub fn to_srgb(&self) -> Color<f32, SRGBColorSpace<f32>> {
    Color::new(self.value.map(|c| {
      if c < 0.0031308 {
        c * 12.92
      } else {
        1.055 * (c.powf(0.41666)) - 0.055
      }
    }))
  }
}
