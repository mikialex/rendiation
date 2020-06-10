pub trait ColorSpace {
  type ContainerValue: Copy + Clone;
}

pub trait RGBColorSpace<T>: ColorSpace{}
pub trait HSLColorSpace<T>: ColorSpace{}
