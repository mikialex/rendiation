use super::{AnyRGBColorSpace, Color, ColorSpace, HSLColorSpace};
use rendiation_math::Vec3;
use std::marker::PhantomData;

pub trait HSLColor<T> {
  fn h(&self) -> T;
  fn s(&self) -> T;
  fn l(&self) -> T;
}

// auto impl <hsl channel fetch> for all color that <marked as hslcolorspace and their value types is vec3<T>>
impl<T: Copy, U: HSLColorSpace<T> + ColorSpace<ContainerValue = Vec3<T>>> HSLColor<T> for Color<U> {
  fn h(&self) -> T {
    self.value.x
  }
  fn s(&self) -> T {
    self.value.y
  }
  fn l(&self) -> T {
    self.value.z
  }
}

pub struct AnyHSLColorSpace<T: Copy + Clone> {
  phantom: PhantomData<T>,
}
impl<T: Copy + Clone> HSLColorSpace<T> for AnyHSLColorSpace<T> {}
impl<T: Copy + Clone> ColorSpace for AnyHSLColorSpace<T> {
  type ContainerValue = Vec3<T>;
}

impl<T: HSLColorSpace<f32> + ColorSpace<ContainerValue = Vec3<f32>>> Color<T> {
  pub fn to_any_rgb(&self) -> Color<AnyRGBColorSpace<f32>> {
    fn hue2rgb(p: f32, q: f32, mut t: f32) -> f32 {
      if t < 0. {
        t += 1.;
      }
      if t > 1. {
        t -= 1.;
      }
      if t < 1. / 6. {
        return p + (q - p) * 6. * t;
      }
      if t < 1. / 2. {
        return q;
      }
      if t < 2. / 3. {
        return p + (q - p) * 6. * (2. / 3. - t);
      }
      return p;
    }

    // h,s,l ranges are in 0.0 - 1.0 // todo standalone clamp impl
    // let h = _Math.euclideanModulo(h, 1);
    // let s = _Math.clamp(s, 0, 1);
    // let l = _Math.clamp(l, 0, 1);

    let h = self.h();
    let s = self.s();
    let l = self.l();

    // let color = Color::<AnyRGBColorSpace<f32>>::new();
    if s == 0. {
      return Color::from_value((l, l, l));
    }

    let p = if l <= 0.5 {
      l * (1. + s)
    } else {
      l + s - (l * s)
    };
    let q = (2. * l) - p;
    let r = hue2rgb(q, p, h + 1. / 3.);
    let g = hue2rgb(q, p, h);
    let b = hue2rgb(q, p, h - 1. / 3.);
    Color::from_value((r, g, b))
  }
}
