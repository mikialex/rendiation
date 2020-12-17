use crate::*;
use std::fmt;
use std::fmt::Debug;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec3<T> {
  pub x: T,
  pub y: T,
  pub z: T,
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec3<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec3<T> {}

impl<T: Scalar> VectorDimension<3> for Vec3<T> {}
impl<T: Scalar> VectorImpl for Vec3<T> {}
impl<T: Scalar> Vector<T> for Vec3<T> {
  #[inline]
  fn dot(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z
  }

  #[inline]
  fn cross(&self, b: Self) -> Self {
    Self {
      x: self.y * b.z - self.z * b.y,
      y: self.z * b.x - self.x * b.z,
      z: self.x * b.y - self.y * b.x,
    }
  }
}

impl<T> Vec3<T>
where
  T: Copy,
{
  #[inline(always)]
  pub fn to_tuple(&self) -> (T, T, T) {
    (self.x, self.y, self.z)
  }
}

impl<T> Vec3<T>
where
  T: Scalar,
{
  /// input: Matrix4 affine matrix
  ///
  /// vector interpreted as a direction
  #[inline]
  pub fn transform_direction(&self, m: Mat4<T>) -> Self {
    Self {
      x: m.a1 * self.x + m.b1 * self.y + m.c1 * self.z,
      y: m.a2 * self.x + m.b2 * self.y + m.c2 * self.z,
      z: m.a3 * self.x + m.b3 * self.y + m.c3 * self.z,
    }
    .normalize()
  }

  #[inline]
  pub fn max_channel(self) -> T {
    self.x.max(self.y).max(self.z)
  }
}

impl<T> Vec3<T> {
  pub fn set(&mut self, x: T, y: T, z: T) -> &Self {
    self.x = x;
    self.y = y;
    self.z = z;
    self
  }
}

impl<T> Math for Vec3<T>
where
  T: Copy + Math,
{
  #[inline]
  fn abs(self) -> Self {
    let mx = self.x.abs();
    let my = self.y.abs();
    let mz = self.z.abs();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn recip(self) -> Self {
    let mx = self.x.recip();
    let my = self.y.recip();
    let mz = self.z.recip();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn sqrt(self) -> Self {
    let mx = self.x.sqrt();
    let my = self.y.sqrt();
    let mz = self.z.sqrt();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn rsqrt(self) -> Self {
    let mx = self.x.rsqrt();
    let my = self.y.rsqrt();
    let mz = self.z.rsqrt();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn sin(self) -> Self {
    let mx = self.x.sin();
    let my = self.y.sin();
    let mz = self.z.sin();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn cos(self) -> Self {
    let mx = self.x.cos();
    let my = self.y.cos();
    let mz = self.z.cos();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn tan(self) -> Self {
    let mx = self.x.tan();
    let my = self.y.tan();
    let mz = self.z.tan();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn sincos(self) -> (Self, Self) {
    let mx = self.x.sincos();
    let my = self.y.sincos();
    let mz = self.z.sincos();
    (
      Self {
        x: mx.0,
        y: my.0,
        z: mz.0,
      },
      Self {
        x: mx.1,
        y: my.1,
        z: mz.1,
      },
    )
  }

  #[inline]
  fn acos(self) -> Self {
    let mx = self.x.acos();
    let my = self.y.acos();
    let mz = self.z.acos();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn asin(self) -> Self {
    let mx = self.x.asin();
    let my = self.y.asin();
    let mz = self.z.asin();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn atan(self) -> Self {
    let mx = self.x.atan();
    let my = self.y.atan();
    let mz = self.z.atan();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn exp(self) -> Self {
    let mx = self.x.exp();
    let my = self.y.exp();
    let mz = self.z.exp();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn exp2(self) -> Self {
    let mx = self.x.exp2();
    let my = self.y.exp2();
    let mz = self.z.exp2();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn log(self, rhs: Self) -> Self {
    let mx = self.x.log(rhs.x);
    let my = self.y.log(rhs.y);
    let mz = self.z.log(rhs.z);
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn log2(self) -> Self {
    let mx = self.x.log2();
    let my = self.y.log2();
    let mz = self.z.log2();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn log10(self) -> Self {
    let mx = self.x.log10();
    let my = self.y.log10();
    let mz = self.z.log10();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn to_radians(self) -> Self {
    let mx = self.x.to_radians();
    let my = self.y.to_radians();
    let mz = self.z.to_radians();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn to_degrees(self) -> Self {
    let mx = self.x.to_degrees();
    let my = self.y.to_degrees();
    let mz = self.z.to_degrees();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn min(self, rhs: Self) -> Self {
    let mx = self.x.min(rhs.x);
    let my = self.y.min(rhs.y);
    let mz = self.z.min(rhs.z);
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn max(self, rhs: Self) -> Self {
    let mx = self.x.max(rhs.x);
    let my = self.y.max(rhs.y);
    let mz = self.z.max(rhs.z);
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn saturate(self) -> Self {
    let mx = self.x.saturate();
    let my = self.y.saturate();
    let mz = self.z.saturate();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn snorm2unorm(self) -> Self {
    let mx = self.x.snorm2unorm();
    let my = self.y.snorm2unorm();
    let mz = self.z.snorm2unorm();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn unorm2snorm(self) -> Self {
    let mx = self.x.unorm2snorm();
    let my = self.y.unorm2snorm();
    let mz = self.z.unorm2snorm();
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }

  #[inline]
  fn clamp(self, minval: Self, maxval: Self) -> Self {
    let mx = self.x.clamp(minval.x, maxval.x);
    let my = self.y.clamp(minval.y, maxval.y);
    let mz = self.z.clamp(minval.z, maxval.z);
    Self {
      x: mx,
      y: my,
      z: mz,
    }
  }
}

impl<T> Zero for Vec3<T>
where
  T: Zero,
{
  #[inline(always)]
  fn zero() -> Self {
    Self {
      x: T::zero(),
      y: T::zero(),
      z: T::zero(),
    }
  }
}

impl<T> One for Vec3<T>
where
  T: One,
{
  #[inline(always)]
  fn one() -> Self {
    Self {
      x: T::one(),
      y: T::one(),
      z: T::one(),
    }
  }
}

impl<T> UnitX for Vec3<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_x() -> Self {
    Self {
      x: T::one(),
      y: T::zero(),
      z: T::zero(),
    }
  }
}

impl<T> UnitY for Vec3<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_y() -> Self {
    Self {
      x: T::zero(),
      y: T::one(),
      z: T::zero(),
    }
  }
}

impl<T> UnitZ for Vec3<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn unit_z() -> Self {
    Self {
      x: T::zero(),
      y: T::one(),
      z: T::zero(),
    }
  }
}

impl<T> fmt::Display for Vec3<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:?}, {:?}, {:?})", self.x, self.y, self.z)
  }
}

impl<T> fmt::Binary for Vec3<T>
where
  T: Scalar,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let len = self.length();
    let decimals = f.precision().unwrap_or(3);
    let string = format!("{:.*?}", decimals, len);
    f.pad_integral(true, "", &string)
  }
}

impl<T> AsRef<Vec3<T>> for Vec3<T> {
  fn as_ref(&self) -> &Vec3<T> {
    self
  }
}

impl<T> AsMut<Vec3<T>> for Vec3<T> {
  fn as_mut(&mut self) -> &mut Vec3<T> {
    self
  }
}

impl Vec3<f32> {
  pub fn apply_mat4(&self, mat: &Mat4<f32>) -> Self {
    let w = 1. / (mat.a4 * self.x + mat.b4 * self.y + mat.c4 * self.z + mat.d4);

    Self {
      x: (mat.a1 * self.x + mat.b1 * self.y + mat.c1 * self.z + mat.d1) * w,
      y: (mat.a2 * self.x + mat.b2 * self.y + mat.c2 * self.z + mat.d2) * w,
      z: (mat.a3 * self.x + mat.b3 * self.y + mat.c3 * self.z + mat.d3) * w,
    }
  }
}
