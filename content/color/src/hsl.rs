use crate::SRGBColor;

#[repr(C)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct HSLColor<T> {
  pub h: T,
  pub s: T,
  pub l: T,
}

unsafe impl<T: bytemuck::Pod> bytemuck::Pod for HSLColor<T> {}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for HSLColor<T> {}

impl From<HSLColor<f32>> for SRGBColor<f32> {
  fn from(color: HSLColor<f32>) -> Self {
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

    let h = color.h;
    let s = color.s;
    let l = color.l;

    // let color = Color::<AnyRGBColorSpace<f32>>::new();
    if s == 0. {
      return Self { r: l, g: l, b: l };
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

    Self { r, g, b }
  }
}
