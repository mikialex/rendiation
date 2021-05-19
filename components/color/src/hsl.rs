use std::marker::PhantomData;

use rendiation_algebra::Vec3;

use crate::{AnyRGBColorSpace, Color, ColorSpace, HSLColorSpace};

pub trait HSLColor<T> {
  fn h(&self) -> T;
  fn s(&self) -> T;
  fn l(&self) -> T;
}

// auto impl <hsl channel fetch> for all color that <marked as hsl colorspace and their value types is vec3<T>>
impl<T: Copy, U: HSLColorSpace<T> + ColorSpace<T, Value = Vec3<T>>> HSLColor<T> for Color<T, U> {
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
impl<T: Copy + Clone> ColorSpace<T> for AnyHSLColorSpace<T> {
  type Value = Vec3<T>;
}

impl<T: HSLColorSpace<f32> + ColorSpace<f32, Value = Vec3<f32>>> Color<f32, T> {
  pub fn to_any_rgb(self) -> Color<f32, AnyRGBColorSpace<f32>> {
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
      p
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
