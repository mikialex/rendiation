use crate::*;
use std::ops::*;

pub trait Arithmetic:
  Copy
  + Clone
  + Add<Self, Output = Self>
  + Sub<Self, Output = Self>
  + Mul<Self, Output = Self>
  + Div<Self, Output = Self>
  + Rem<Self, Output = Self>
  + AddAssign<Self>
  + SubAssign<Self>
  + MulAssign<Self>
  + DivAssign<Self>
  + Neg<Output = Self>
  + PartialEq
  + PartialOrd
  + num_traits::identities::One
  + num_traits::identities::Zero
  + Two
  + Three
{
}

impl Arithmetic for f32 {}
impl Arithmetic for f64 {}
impl Arithmetic for i32 {}
impl Arithmetic for i64 {}
