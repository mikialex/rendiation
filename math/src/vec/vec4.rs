use crate::*;
use std::fmt;
use std::fmt::Debug;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec4<T> {
  pub x: T,
  pub y: T,
  pub z: T,
  pub w: T,
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec4<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec4<T> {}

impl<T> Vec4<T>
where
  T: Copy,
{
  #[inline(always)]
  pub fn to_tuple(&self) -> (T, T, T, T) {
    (self.x, self.y, self.z, self.w)
  }
}

impl<T> Vec4<T>
where
  T: Arithmetic + Math,
{
  #[inline]
  pub fn dot(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w
  }
  #[inline]
  pub fn cross(&self, b: Self) -> Self {
    Vec4 {
      x: self.w * b.x + self.x * b.w + self.z * b.y - self.y * b.z,
      y: self.w * b.y + self.y * b.w + self.x * b.z - self.z * b.x,
      z: self.w * b.z + self.z * b.w + self.y * b.x - self.x * b.y,
      w: self.w * b.w - self.x * b.x - self.y * b.y - self.z * b.z,
    }
  }
  #[inline]
  pub fn length2(&self) -> T {
    self.dot(*self)
  }
  #[inline]
  pub fn length(&self) -> T {
    self.length2().sqrt()
  }
  #[inline]
  pub fn distance(&self, b: Self) -> T {
    (*self - b).length()
  }

  #[inline]
  pub fn normalize(&self) -> Self {
    let mag_sq = self.length2();
    if mag_sq > T::zero() {
      let inv_sqrt = T::one() / mag_sq.sqrt();
      return *self * inv_sqrt;
    }

    *self
  }
}

impl<T> Math for Vec4<T>
where
  T: Copy + Math,
{
  #[inline]
  fn abs(self) -> Self {
    let mx = self.x.abs();
    let my = self.y.abs();
    let mz = self.z.abs();
    let mw = self.w.abs();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn recip(self) -> Self {
    let mx = self.x.recip();
    let my = self.y.recip();
    let mz = self.z.recip();
    let mw = self.w.recip();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn sqrt(self) -> Self {
    let mx = self.x.sqrt();
    let my = self.y.sqrt();
    let mz = self.z.sqrt();
    let mw = self.w.sqrt();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn rsqrt(self) -> Self {
    let mx = self.x.rsqrt();
    let my = self.y.rsqrt();
    let mz = self.z.rsqrt();
    let mw = self.w.rsqrt();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn sin(self) -> Self {
    let mx = self.x.sin();
    let my = self.y.sin();
    let mz = self.z.sin();
    let mw = self.w.sin();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn cos(self) -> Self {
    let mx = self.x.cos();
    let my = self.y.cos();
    let mz = self.z.cos();
    let mw = self.w.cos();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn tan(self) -> Self {
    let mx = self.x.tan();
    let my = self.y.tan();
    let mz = self.z.tan();
    let mw = self.w.tan();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn sincos(self) -> (Vec4<T>, Vec4<T>) {
    let mx = self.x.sincos();
    let my = self.y.sincos();
    let mz = self.z.sincos();
    let mw = self.w.sincos();
    (
      Self {
        x: mx.0,
        y: my.0,
        z: mz.0,
        w: mw.0,
      },
      Self {
        x: mx.1,
        y: my.1,
        z: mz.1,
        w: mw.1,
      },
    )
  }

  #[inline]
  fn acos(self) -> Self {
    let mx = self.x.acos();
    let my = self.y.acos();
    let mz = self.z.acos();
    let mw = self.w.acos();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn asin(self) -> Self {
    let mx = self.x.asin();
    let my = self.y.asin();
    let mz = self.z.asin();
    let mw = self.w.asin();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn atan(self) -> Self {
    let mx = self.x.atan();
    let my = self.y.atan();
    let mz = self.z.atan();
    let mw = self.w.atan();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn exp(self) -> Self {
    let mx = self.x.exp();
    let my = self.y.exp();
    let mz = self.z.exp();
    let mw = self.w.exp();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn exp2(self) -> Self {
    let mx = self.x.exp2();
    let my = self.y.exp2();
    let mz = self.z.exp2();
    let mw = self.w.exp2();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn log(self, _rhs: Self) -> Self {
    let mx = self.x.log(_rhs.x);
    let my = self.y.log(_rhs.y);
    let mz = self.z.log(_rhs.z);
    let mw = self.w.log(_rhs.w);
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn log2(self) -> Self {
    let mx = self.x.log2();
    let my = self.y.log2();
    let mz = self.z.log2();
    let mw = self.w.log2();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn log10(self) -> Self {
    let mx = self.x.log10();
    let my = self.y.log10();
    let mz = self.z.log10();
    let mw = self.w.log10();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn to_radians(self) -> Self {
    let mx = self.x.to_radians();
    let my = self.y.to_radians();
    let mz = self.z.to_radians();
    let mw = self.w.to_radians();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn to_degrees(self) -> Self {
    let mx = self.x.to_degrees();
    let my = self.y.to_degrees();
    let mz = self.z.to_degrees();
    let mw = self.w.to_degrees();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn min(self, _rhs: Self) -> Self {
    let mx = self.x.min(_rhs.x);
    let my = self.y.min(_rhs.y);
    let mz = self.z.min(_rhs.z);
    let mw = self.w.min(_rhs.x);
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn max(self, _rhs: Self) -> Self {
    let mx = self.x.max(_rhs.x);
    let my = self.y.max(_rhs.y);
    let mz = self.z.max(_rhs.z);
    let mw = self.w.max(_rhs.w);
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn saturate(self) -> Self {
    let mx = self.x.saturate();
    let my = self.y.saturate();
    let mz = self.z.saturate();
    let mw = self.w.saturate();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn snorm2unorm(self) -> Self {
    let mx = self.x.snorm2unorm();
    let my = self.y.snorm2unorm();
    let mz = self.z.snorm2unorm();
    let mw = self.w.snorm2unorm();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn unorm2snorm(self) -> Self {
    let mx = self.x.unorm2snorm();
    let my = self.y.unorm2snorm();
    let mz = self.z.unorm2snorm();
    let mw = self.w.unorm2snorm();
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }

  #[inline]
  fn clamp(self, minval: Self, maxval: Self) -> Self {
    let mx = self.x.clamp(minval.x, maxval.x);
    let my = self.y.clamp(minval.y, maxval.y);
    let mz = self.z.clamp(minval.z, maxval.z);
    let mw = self.w.clamp(minval.w, maxval.w);
    Self {
      x: mx,
      y: my,
      z: mz,
      w: mw,
    }
  }
}

impl<T: Arithmetic> Lerp<T> for Vec4<T> {
  #[inline(always)]
  fn lerp(self, b: Self, t: T) -> Self {
    return self * (T::one() - t) + b * t;
  }
}

impl<T> Slerp<T> for Vec4<T>
where
  T: Arithmetic + Math,
{
  fn slerp(self, other: Self, factor: T) -> Self {
    let dot = self.dot(other);

    let s = T::one() - factor;
    let t = if dot > T::zero() { factor } else { -factor };
    let q = self * s + other * t;

    q.normalize()
  }
}

impl<T> Zero for Vec4<T>
where
  T: Zero,
{
  #[inline(always)]
  fn zero() -> Self {
    Self {
      x: T::zero(),
      y: T::zero(),
      z: T::zero(),
      w: T::zero(),
    }
  }
}

impl<T> One for Vec4<T>
where
  T: One,
{
  #[inline(always)]
  fn one() -> Self {
    Self {
      x: T::one(),
      y: T::one(),
      z: T::one(),
      w: T::one(),
    }
  }
}

impl<T> UnitX for Vec4<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_x() -> Self {
    Self {
      x: T::one(),
      y: T::zero(),
      z: T::zero(),
      w: T::one(),
    }
  }
}

impl<T> UnitY for Vec4<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_y() -> Self {
    Self {
      x: T::zero(),
      y: T::one(),
      z: T::zero(),
      w: T::one(),
    }
  }
}

impl<T> UnitZ for Vec4<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_z() -> Self {
    Self {
      x: T::zero(),
      y: T::one(),
      z: T::zero(),
      w: T::one(),
    }
  }
}

impl<T> UnitW for Vec4<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_w() -> Self {
    Self {
      x: T::zero(),
      y: T::zero(),
      z: T::zero(),
      w: T::one(),
    }
  }
}

impl<T> fmt::Display for Vec4<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "({:?}, {:?}, {:?}, {:?})",
      self.x, self.y, self.z, self.w
    )
  }
}

impl<T> fmt::Binary for Vec4<T>
where
  T: Arithmetic + Math + Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let len = self.length();
    let decimals = f.precision().unwrap_or(3);
    let string = format!("{:.*?}", decimals, len);
    f.pad_integral(true, "", &string)
  }
}

impl<T> AsRef<Vec4<T>> for Vec4<T> {
  fn as_ref(&self) -> &Vec4<T> {
    self
  }
}

impl<T> AsMut<Vec4<T>> for Vec4<T> {
  fn as_mut(&mut self) -> &mut Vec4<T> {
    self
  }
}
