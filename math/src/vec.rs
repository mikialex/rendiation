use std::ops::{Add, Sub, Mul, Div, Rem, Neg, Not, BitAnd, BitOr, BitXor};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use std::{f32, f64};
use std::fmt::Debug;

use super::consts::*;

pub trait Vec:
	Debug + Copy + Clone
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
	+ Cmp
	+ One + Two + Zero + Half
{
}

impl Vec for f32 {}
impl Vec for f64 {}


pub trait Lerp<T> 
{
	fn lerp(self, rhs: Self, t:T) -> Self; 
}

impl Lerp<f32> for f32
{
	#[inline(always)]
    fn lerp(self, b: Self, t: f32) -> Self 
    {
        return self * (1.0 - t) + b * t;
    }
}

impl Lerp<f64> for f64
{
	#[inline(always)]
    fn lerp(self, b: Self, t: Self) -> Self 
    {
        return self * (1.0 - t) + b * t;
    }
}

pub trait Slerp<T> 
{
	fn slerp(self, rhs: Self, t:T) -> Self; 
}

pub trait Cmp 
{
    type Bool: Copy
               + Not<Output = Self::Bool>
               + BitAnd<Self::Bool, Output = Self::Bool>
               + BitOr<Self::Bool, Output = Self::Bool>
               + BitXor<Self::Bool, Output = Self::Bool>;

    fn eq(self, rhs: Self) -> bool;
    fn ne(self, rhs: Self) -> bool;
    fn gt(self, rhs: Self) -> bool;
    fn lt(self, rhs: Self) -> bool;
    fn ge(self, rhs: Self) -> bool;
    fn le(self, rhs: Self) -> bool;
}

impl Cmp for f32 
{
    type Bool = bool;

    #[inline(always)] fn eq(self, rhs: Self) -> bool { self == rhs }
    #[inline(always)] fn ne(self, rhs: Self) -> bool { self != rhs }
    #[inline(always)] fn gt(self, rhs: Self) -> bool { self > rhs }
    #[inline(always)] fn lt(self, rhs: Self) -> bool { self < rhs }
    #[inline(always)] fn ge(self, rhs: Self) -> bool { self >= rhs }
    #[inline(always)] fn le(self, rhs: Self) -> bool { self <= rhs }
}

impl Cmp for f64 
{
    type Bool = bool;

    #[inline(always)] fn eq(self, rhs: Self) -> bool { self == rhs }
    #[inline(always)] fn ne(self, rhs: Self) -> bool { self != rhs }
    #[inline(always)] fn gt(self, rhs: Self) -> bool { self > rhs }
    #[inline(always)] fn lt(self, rhs: Self) -> bool { self < rhs }
    #[inline(always)] fn ge(self, rhs: Self) -> bool { self >= rhs }
    #[inline(always)] fn le(self, rhs: Self) -> bool { self <= rhs }
}