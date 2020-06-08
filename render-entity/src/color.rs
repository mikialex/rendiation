use rendiation_math::Vec3;

pub trait ColorSpace<T> {
  type ContainerValue;
}

pub struct Color<T = f32, S: ColorSpace<T> = SRGBColorSpace> {
  value: S::ContainerValue,
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
