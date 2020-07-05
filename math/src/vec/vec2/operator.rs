use crate::Vec2;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

impl<T> Neg for Vec2<T>
where
  T: Neg<Output = T>,
{
  type Output = Self;

  fn neg(self) -> Self {
    Self {
      x: -self.x,
      y: -self.y,
    }
  }
}

impl<T> Add for Vec2<T>
where
  T: Add<Output = T>,
{
  type Output = Self;

  fn add(self, other: Self) -> Self {
    Self {
      x: self.x + other.x,
      y: self.y + other.y,
    }
  }
}

impl<T> Sub for Vec2<T>
where
  T: Sub<Output = T>,
{
  type Output = Self;

  fn sub(self, other: Self) -> Self {
    Self {
      x: self.x - other.x,
      y: self.y - other.y,
    }
  }
}

impl<T> Mul<T> for Vec2<T>
where
  T: Mul<Output = T> + Copy,
{
  type Output = Self;

  fn mul(self, s: T) -> Self {
    Self {
      x: self.x * s,
      y: self.y * s,
    }
  }
}

impl<T> Mul for Vec2<T>
where
  T: Mul<Output = T>,
{
  type Output = Self;

  fn mul(self, other: Self) -> Self {
    Self {
      x: self.x * other.x,
      y: self.y * other.y,
    }
  }
}

impl<T> Div<T> for Vec2<T>
where
  T: Div<Output = T> + Copy,
{
  type Output = Self;

  fn div(self, s: T) -> Self {
    Self {
      x: self.x / s,
      y: self.y / s,
    }
  }
}

impl<T> Div for Vec2<T>
where
  T: Div<Output = T>,
{
  type Output = Self;

  fn div(self, other: Self) -> Self {
    Self {
      x: self.x / other.x,
      y: self.y / other.y,
    }
  }
}

impl<T> AddAssign for Vec2<T>
where
  T: AddAssign<T>,
{
  fn add_assign(&mut self, other: Self) {
    self.x += other.x;
    self.y += other.y;
  }
}

impl<T> SubAssign for Vec2<T>
where
  T: SubAssign<T>,
{
  fn sub_assign(&mut self, other: Self) {
    self.x -= other.x;
    self.y -= other.y;
  }
}

impl<T> MulAssign for Vec2<T>
where
  T: MulAssign<T>,
{
  fn mul_assign(&mut self, other: Self) {
    self.x *= other.x;
    self.y *= other.y;
  }
}

impl<T> MulAssign<T> for Vec2<T>
where
  T: MulAssign<T> + Copy,
{
  fn mul_assign(&mut self, s: T) {
    self.x *= s;
    self.y *= s;
  }
}

impl<'a, T> MulAssign<&'a T> for Vec2<T>
where
  T: MulAssign<T> + Copy,
{
  fn mul_assign(&mut self, other: &'a T) {
    self.x *= *other;
    self.y *= *other;
  }
}

impl<T> DivAssign for Vec2<T>
where
  T: DivAssign<T>,
{
  fn div_assign(&mut self, other: Self) {
    self.x /= other.x;
    self.y /= other.y;
  }
}

impl<T> DivAssign<T> for Vec2<T>
where
  T: DivAssign<T> + Copy,
{
  fn div_assign(&mut self, s: T) {
    self.x /= s;
    self.y /= s;
  }
}

impl<'a, T> DivAssign<&'a T> for Vec2<T>
where
  T: DivAssign<T> + Copy,
{
  fn div_assign(&mut self, s: &'a T) {
    self.x /= *s;
    self.y /= *s;
  }
}
