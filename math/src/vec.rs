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
	+ One + Two + Zero + OneHalf
{
}

impl Vec for f32 {}
impl Vec for f64 {}

pub trait Math: Sized
{
	fn abs(self) -> Self;
	fn recip(self) -> Self;
	fn sqrt(self) -> Self;
	fn rsqrt(self) -> Self;
	fn sin(self) -> Self;
	fn cos(self) -> Self;
	fn tan(self) -> Self;
	fn sincos(self) -> (Self, Self);
	fn acos(self) -> Self;
	fn asin(self) -> Self;
	fn atan(self) -> Self;
	fn exp(self) -> Self;
	fn exp2(self) -> Self;
	fn log(self, rhs: Self) -> Self;
	fn log2(self) -> Self;
	fn log10(self) -> Self;
	fn to_radians(self) -> Self;
	fn to_degrees(self) -> Self;
	fn min(self, rhs: Self) -> Self;
	fn max(self, rhs: Self) -> Self;
	fn saturate(self) -> Self;
	fn snorm2unorm(self) -> Self;
	fn unorm2snorm(self) -> Self;
	fn clamp(self, minval: Self, maxval: Self) -> Self;
}

impl Math for f32 
{
	#[inline(always)] fn abs(self) -> Self { f32::abs(self) }
	#[inline(always)] fn recip(self) -> Self { f32::recip(self) }
	#[inline(always)] fn sqrt(self) -> Self { f32::sqrt(self) }
	#[inline(always)] fn rsqrt(self) -> Self { f32::recip(f32::sqrt(self)) }
	#[inline(always)] fn sin(self) -> Self { f32::sin(self) }
	#[inline(always)] fn cos(self) -> Self { f32::cos(self) }
	#[inline(always)] fn tan(self) -> Self { f32::tan(self) }
	#[inline(always)] fn sincos(self) -> (f32, f32) { f32::sin_cos(self) }
	#[inline(always)] fn acos(self) -> Self { f32::acos(self) }
	#[inline(always)] fn asin(self) -> Self { f32::asin(self) }
	#[inline(always)] fn atan(self) -> Self { f32::atan(self) }
	#[inline(always)] fn exp(self) -> Self { f32::exp(self) }
	#[inline(always)] fn exp2(self) -> Self { f32::exp2(self) }
	#[inline(always)] fn log(self, y:f32) -> Self { f32::log(self, y) }
	#[inline(always)] fn log2(self) -> Self { f32::log2(self) }
	#[inline(always)] fn log10(self) -> Self { f32::log10(self) }
	#[inline(always)] fn to_radians(self) -> Self { f32::to_radians(self) }
	#[inline(always)] fn to_degrees(self) -> Self { f32::to_degrees(self) }
	#[inline(always)] fn min(self, y: f32) -> Self { f32::min(self, y) }
	#[inline(always)] fn max(self, y: f32) -> Self { f32::max(self, y) }
	#[inline(always)] fn saturate(self) -> Self { f32::min(1.0, f32::max(0.0, self)) }
	#[inline(always)] fn snorm2unorm(self) -> Self { self * 0.5 + 0.5 }	
	#[inline(always)] fn unorm2snorm(self) -> Self { self * 2.0 + 1.0 }
	#[inline(always)] fn clamp(self, minval: f32, maxval: f32) -> Self { f32::min(maxval, f32::max(minval, self)) }
}

impl Math for f64 
{
	#[inline(always)] fn abs(self) -> Self { f64::abs(self) }
	#[inline(always)] fn recip(self) -> Self { f64::recip(self) }
	#[inline(always)] fn sqrt(self) -> Self { f64::sqrt(self) }
	#[inline(always)] fn rsqrt(self) -> Self { f64::recip(f64::sqrt(self)) }
	#[inline(always)] fn sin(self) -> Self { f64::sin(self) }
	#[inline(always)] fn cos(self) -> Self { f64::cos(self) }
	#[inline(always)] fn tan(self) -> Self { f64::tan(self) }
	#[inline(always)] fn sincos(self) -> (f64, f64) { f64::sin_cos(self) }
	#[inline(always)] fn acos(self) -> Self { f64::acos(self) }
	#[inline(always)] fn asin(self) -> Self { f64::asin(self) }
	#[inline(always)] fn atan(self) -> Self { f64::atan(self) }
	#[inline(always)] fn exp(self) -> Self { f64::exp(self) }
	#[inline(always)] fn exp2(self) -> Self { f64::exp2(self) }
	#[inline(always)] fn log(self, y:f64) -> Self { f64::log(self, y) }
	#[inline(always)] fn log2(self) -> Self { f64::log2(self) }
	#[inline(always)] fn log10(self) -> Self { f64::log10(self) }
	#[inline(always)] fn to_radians(self) -> Self { f64::to_radians(self) }
	#[inline(always)] fn to_degrees(self) -> Self { f64::to_degrees(self) }
	#[inline(always)] fn min(self, y: f64) -> Self { f64::min(self, y) }
	#[inline(always)] fn max(self, y: f64) -> Self { f64::max(self, y) }
	#[inline(always)] fn saturate(self) -> Self { f64::min(1.0, f64::max(0.0, self)) }
	#[inline(always)] fn snorm2unorm(self) -> Self { self * 0.5 + 0.5 }	
	#[inline(always)] fn unorm2snorm(self) -> Self { self * 2.0 + 1.0 }
	#[inline(always)] fn clamp(self, minval: f64, maxval: f64) -> Self { f64::min(maxval, f64::max(minval, self)) }
}

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