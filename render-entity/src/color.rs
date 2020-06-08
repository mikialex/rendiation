use rendiation_math::Vec3;
use std::ops::Mul;

pub trait ColorSpace<T> {
  type ContainerValue;
}

#[derive(Debug, Copy, Clone)]
pub struct Color<T = f32, S: ColorSpace<T> = SRGBColorSpace> {
  value: S::ContainerValue,
}

// multiply scalar
impl<T, S: ColorSpace<T>, U> Mul<U> for Color<T, S>
where
  S::ContainerValue: Mul<U, Output = S::ContainerValue> + Copy,
{
  type Output = Self;

  fn mul(self, other: U) -> Self {
    Self {
      value: self.value * other,
    }
  }
}

impl<T, S: ColorSpace<T>> Color<T, S> {
  pub fn new(value: S::ContainerValue) -> Self {
    Self { value }
  }
}

pub struct SRGBColorSpace {}
pub struct SRGBColorSpaceChannelValue<T>(pub T);

impl<T> ColorSpace<T> for SRGBColorSpace {
  type ContainerValue = Vec3<T>;
}

impl<T: Copy> Color<T, SRGBColorSpace> {
  pub fn r(&self) -> SRGBColorSpaceChannelValue<T> {
    SRGBColorSpaceChannelValue(self.value.x)
  }
  pub fn g(&self) -> SRGBColorSpaceChannelValue<T> {
    SRGBColorSpaceChannelValue(self.value.y)
  }
  pub fn b(&self) -> SRGBColorSpaceChannelValue<T> {
    SRGBColorSpaceChannelValue(self.value.z)
  }
}

impl Color<f32, SRGBColorSpace> {
  pub fn to_linear_rgb(&self) -> Color<f32, LinearRGBColorSpace> {
    Color::new(self.value.map(|c| {
      if c < 0.04045 {
        c * 0.0773993808
      } else {
        (c * 0.9478672986 + 0.0521327014).powf(2.4)
      }
    }))
  }
}

pub struct LinearRGBColorSpace {}
pub struct LinearColorSpaceChannelValue<T>(pub T);

impl<T> ColorSpace<T> for LinearRGBColorSpace {
  type ContainerValue = Vec3<T>;
}

impl<T: Copy> Color<T, LinearRGBColorSpace> {
  pub fn r(&self) -> LinearColorSpaceChannelValue<T> {
    LinearColorSpaceChannelValue(self.value.x)
  }
  pub fn g(&self) -> LinearColorSpaceChannelValue<T> {
    LinearColorSpaceChannelValue(self.value.y)
  }
  pub fn b(&self) -> LinearColorSpaceChannelValue<T> {
    LinearColorSpaceChannelValue(self.value.z)
  }
}

impl Color<f32, LinearRGBColorSpace> {
  pub fn to_linear_rgb(&self) -> Color<f32, SRGBColorSpace> {
    Color::new(self.value.map(|c| {
      if c < 0.0031308 {
        c * 12.92
      } else {
        1.055 * (c.powf(0.41666)) - 0.055
      }
    }))
  }
}
