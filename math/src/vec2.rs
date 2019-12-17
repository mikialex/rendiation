use std::fmt;
use std::fmt::Debug;
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use super::vec::{Vec, Math, Lerp, Slerp};
use super::vec3::Vec3;
use super::vec4::Vec4;
use super::consts::{Zero, One, UnitX, UnitY};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec2<T> 
{
	pub x: T,
	pub y: T,
}

impl<T> Neg for Vec2<T> where T:Neg<Output=T> 
{
	type Output = Self;

	fn neg(self) -> Self
	{
		Self
		{
			x: -self.x, 
			y: -self.y 
		}
	}
}

impl<T> Add for Vec2<T> where T:Add<Output=T>
{
	type Output = Self;

	fn add(self, other: Self) -> Self
	{
		Self
		{ 
			x: self.x + other.x, 
			y: self.y + other.y
		}
	}
}

impl<T> Sub for Vec2<T> where T:Sub<Output=T>
{
	type Output = Self;

	fn sub(self, other: Self) -> Self
	{
		Self
		{ 
			x: self.x - other.x, 
			y: self.y - other.y
		}
	}
}

impl<T> Mul<T> for Vec2<T> where T:Mul<Output=T> + Copy
{
	type Output = Self;

	fn mul(self, s: T) -> Self
	{
		Self
		{
			x:self.x * s,
			y:self.y * s,
		}
	}
}

impl<T> Mul for Vec2<T> where T:Mul<Output=T>
{
	type Output = Self;

	fn mul(self, other: Self) -> Self
	{
		Self
		{ 
			x: self.x * other.x, 
			y: self.y * other.y
		}
	}
}

impl<T> Div<T> for Vec2<T> where T:Div<Output=T> + Copy
{
	type Output = Self;

	fn div(self, s: T) -> Self
	{
		Self
		{
			x:self.x / s,
			y:self.y / s,
		}
	}
}

impl<T> Div for Vec2<T> where T:Div<Output=T>
{
	type Output = Self;

	fn div(self, other: Self) -> Self
	{
		Self
		{ 
			x: self.x / other.x, 
			y: self.y / other.y
		}
	}
}

impl<T> AddAssign for Vec2<T> where T:AddAssign<T>
{
	fn add_assign(&mut self, other: Self)
	{
		self.x += other.x;
		self.y += other.y; 
	}
}

impl<T> SubAssign for Vec2<T> where T:SubAssign<T>
{
	fn sub_assign(&mut self, other: Self)
	{
		self.x -= other.x;
		self.y -= other.y; 
	}
}

impl<T> MulAssign for Vec2<T> where T: MulAssign<T>
{
	fn mul_assign(&mut self, other: Self)
	{
		self.x *= other.x;
		self.y *= other.y; 
	}
}

impl<T> MulAssign<T> for Vec2<T> where T:MulAssign<T> + Copy
{
	fn mul_assign(&mut self, s: T)
	{
		self.x *= s;
		self.y *= s;
	}
}

impl<'a, T> MulAssign<&'a T> for Vec2<T> where T:MulAssign<T> + Copy
{
	fn mul_assign(&mut self, other: &'a T)
	{
		self.x *= *other;
		self.y *= *other;
	}
}

impl<T> DivAssign for Vec2<T> where T:DivAssign<T>
{
	fn div_assign(&mut self, other: Self)
	{
		self.x /= other.x;
		self.y /= other.y; 
	}
}

impl<T> DivAssign<T> for Vec2<T> where T:DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: T)
	{
		self.x /= s;
		self.y /= s;
	}
}

impl<'a, T> DivAssign<&'a T> for Vec2<T> where T:DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: &'a T)
	{
		self.x /= *s;
		self.y /= *s;
	}
}

impl<T> Vec2<T> where T: Copy
{
	/// Creates a new Vec2 from multiple components
	#[inline(always)]
	pub fn new(x: T, y: T) -> Self { Self { x, y } }

	/// return the length of element
	#[inline(always)]
	pub fn len() -> usize 
	{ 
		return 2; 
	}

	#[inline(always)]
	pub fn to_tuple(&self) -> (T, T)
	{
		(self.x, self.y)
	}

	#[inline(always)]
	pub fn xx(&self) -> Self { Self::new(self.x, self.x) }
	#[inline(always)]
	pub fn xy(&self) -> Self { Self::new(self.x, self.y) }
	#[inline(always)]
	pub fn yx(&self) -> Self { Self::new(self.y, self.x) }
	#[inline(always)]
	pub fn yy(&self) -> Self { Self::new(self.y, self.y) }
	#[inline(always)]
	pub fn xxx(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.x) }
	#[inline(always)]
	pub fn xxy(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.y) }
	#[inline(always)]
	pub fn xyx(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.x) }
	#[inline(always)]
	pub fn xyy(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.y) }
	#[inline(always)]
	pub fn yxx(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.x) }
	#[inline(always)]
	pub fn yxy(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.y) }
	#[inline(always)]
	pub fn yyx(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.x) }
	#[inline(always)]
	pub fn yyy(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.y) }
	#[inline(always)]
	pub fn xxxx(&self) -> Vec4<T> { Vec4::new(self.x, self.x, self.x, self.x) }
	#[inline(always)]
	pub fn xxyx(&self) -> Vec4<T> { Vec4::new(self.x, self.x, self.y, self.x) }
	#[inline(always)]
	pub fn xyxx(&self) -> Vec4<T> { Vec4::new(self.x, self.y, self.x, self.x) }
	#[inline(always)]
	pub fn xyyx(&self) -> Vec4<T> { Vec4::new(self.x, self.y, self.y, self.x) }
	#[inline(always)]
	pub fn xxxy(&self) -> Vec4<T> { Vec4::new(self.x, self.x, self.x, self.y) }
	#[inline(always)]
	pub fn xxyy(&self) -> Vec4<T> { Vec4::new(self.x, self.x, self.y, self.y) }
	#[inline(always)]
	pub fn xyxy(&self) -> Vec4<T> { Vec4::new(self.x, self.y, self.x, self.y) }
	#[inline(always)]
	pub fn xyyy(&self) -> Vec4<T> { Vec4::new(self.x, self.y, self.y, self.y) }
	#[inline(always)]
	pub fn yxxx(&self) -> Vec4<T> { Vec4::new(self.y, self.x, self.x, self.x) }
	#[inline(always)]
	pub fn yxyx(&self) -> Vec4<T> { Vec4::new(self.y, self.x, self.y, self.x) }
	#[inline(always)]
	pub fn yyxx(&self) -> Vec4<T> { Vec4::new(self.y, self.y, self.x, self.x) }
	#[inline(always)]
	pub fn yyyx(&self) -> Vec4<T> { Vec4::new(self.y, self.y, self.y, self.x) }
	#[inline(always)]
	pub fn yxxy(&self) -> Vec4<T> { Vec4::new(self.y, self.x, self.x, self.y) }
	#[inline(always)]
	pub fn yxyy(&self) -> Vec4<T> { Vec4::new(self.y, self.x, self.y, self.y) }
	#[inline(always)]
	pub fn yyxy(&self) -> Vec4<T> { Vec4::new(self.y, self.y, self.x, self.y) }
	#[inline(always)]
	pub fn yyyy(&self) -> Vec4<T> { Vec4::new(self.y, self.y, self.y, self.y) }
}

impl<T> Vec2<T> where T:Vec + Math
{
	#[inline]
	pub fn dot(&self, b: Self) -> T 
	{
		return self.x * b.x + self.y * b.y;
	}

	#[inline]	
	pub fn cross(&self, b: Self) -> Self
	{
		Self
		{
			x:self.y * b.x - self.x * b.y,
			y:self.x * b.y - self.y * b.x
		}
	}

	#[inline]
	pub fn length2(&self) -> T 
	{
		return self.dot(*self);
	}

	#[inline]	
	pub fn length(&self) -> T 
	{
		return self.length2().sqrt();
	}

	#[inline]	
	pub fn distance(&self, b: Self) -> T 
	{
		return (*self - b).length();
	}

	#[inline]
	pub fn normalize(&self) -> Self 
	{
		let mag_sq = self.length2();
		if mag_sq.gt(T::zero())
		{
			let inv_sqrt = T::one() / mag_sq.sqrt();
			return *self * inv_sqrt;
		}

		return *self;
	}
}

impl<T> Math for Vec2<T> where T:Copy + Math
{
	#[inline]
	fn abs(self) -> Self
	{
		let mx = self.x.abs();
		let my = self.y.abs();
		Self { x: mx, y: my }
	}

	#[inline]
	fn recip(self) -> Self
	{
		let mx = self.x.recip();
		let my = self.y.recip();
		Self { x: mx, y: my }
	}

	#[inline]
	fn sqrt(self) -> Self
	{
		let mx = self.x.sqrt();
		let my = self.y.sqrt();
		Self { x: mx, y: my }
	}

	#[inline]
	fn rsqrt(self) -> Self
	{
		let mx = self.x.rsqrt();
		let my = self.y.rsqrt();
		Self { x: mx, y: my }
	}

	#[inline]
	fn sin(self) -> Self
	{
		let mx = self.x.sin();
		let my = self.y.sin();
		Self { x: mx, y: my }
	}

	#[inline]
	fn cos(self) -> Self
	{
		let mx = self.x.cos();
		let my = self.y.cos();
		Self { x: mx, y: my }
	}

	#[inline]
	fn tan(self) -> Self
	{
		let mx = self.x.tan();
		let my = self.y.tan();
		Self { x: mx, y: my }
	}

	#[inline]
	fn sincos(self) -> (Self, Self)
	{
		let mx = self.x.sincos();
		let my = self.y.sincos();
		(
			Self { x: mx.0, y: my.0 },
			Self { x: mx.1, y: my.1 }
		)
	}

	#[inline]
	fn acos(self) -> Self
	{
		let mx = self.x.acos();
		let my = self.y.acos();
		Self { x: mx, y: my }
	}

	#[inline]
	fn asin(self) -> Self
	{
		let mx = self.x.asin();
		let my = self.y.asin();
		Self { x: mx, y: my }
	}

	#[inline]
	fn atan(self) -> Self
	{
		let mx = self.x.atan();
		let my = self.y.atan();
		Self { x: mx, y: my }
	}

	#[inline]
	fn exp(self) -> Self
	{
		let mx = self.x.exp();
		let my = self.y.exp();
		Self { x: mx, y: my }
	}

	#[inline]
	fn exp2(self) -> Self
	{
		let mx = self.x.exp2();
		let my = self.y.exp2();
		Self { x: mx, y: my }
	}

	#[inline]
	fn log(self, _rhs:Self) -> Self
	{
		let mx = self.x.log(_rhs.x);
		let my = self.y.log(_rhs.y);
		Self { x: mx, y: my }
	}

	#[inline]
	fn log2(self) -> Self
	{
		let mx = self.x.log2();
		let my = self.y.log2();
		Self { x: mx, y: my }
	}

	#[inline]
	fn log10(self) -> Self
	{
		let mx = self.x.log10();
		let my = self.y.log10();
		Self { x: mx, y: my }
	}

	#[inline]
	fn to_radians(self) -> Self
	{
		let mx = self.x.to_radians();
		let my = self.y.to_radians();
		Self { x: mx, y: my }
	}

	#[inline]
	fn to_degrees(self) -> Self
	{
		let mx = self.x.to_degrees();
		let my = self.y.to_degrees();
		Self { x: mx, y: my }
	}

	#[inline]
	fn min(self, _rhs: Self) -> Self
	{
		let mx = self.x.min(_rhs.x);
		let my = self.y.min(_rhs.y);
		Self { x: mx, y: my }
	}

	#[inline]
	fn max(self, _rhs: Self) -> Self
	{
		let mx = self.x.max(_rhs.x);
		let my = self.y.max(_rhs.y);
		Self { x: mx, y: my }
	}

	#[inline]
	fn saturate(self) -> Self
	{
		let mx = self.x.saturate();
		let my = self.y.saturate();
		Self { x: mx, y: my }
	}

	#[inline]
	fn snorm2unorm(self) -> Self
	{
		let mx = self.x.snorm2unorm();
		let my = self.y.snorm2unorm();
		Self { x: mx, y: my }
	}

	#[inline]
	fn unorm2snorm(self) -> Self
	{
		let mx = self.x.unorm2snorm();
		let my = self.y.unorm2snorm();
		Self { x: mx, y: my }
	}

	#[inline]
	fn clamp(self, minval: Self, maxval: Self) -> Self
	{
		let mx = self.x.clamp(minval.x, maxval.x);
		let my = self.y.clamp(minval.y, maxval.y);
		Self { x: mx, y: my }
	}
}

impl<T> Lerp<T> for Vec2<T> where T: Copy + One + Mul<Output=T> + Add<Output=T> + Sub<Output=T>
{
	#[inline(always)]
	fn lerp(self, b: Self, t: T) -> Self 
	{
		return self*(T::one() - t) + b*t;
	}
}

impl<T> Slerp<T> for Vec2<T> where T: Vec + Math
{
	fn slerp(self, other: Self, factor: T) -> Self 
	{
		let dot = self.dot(other);

		let s = T::one() - factor;
		let t = if dot.gt(T::zero()) { factor } else { -factor };
		let q = self * s + other * t;

		q.normalize()
	}
}

impl<T> Zero for Vec2<T> where T:Zero
{
	#[inline(always)]
	fn zero() -> Self
	{
		Self
		{
			x: T::zero(), y: T::zero() 
		}
	}
}

impl<T> One for Vec2<T> where T:One
{
	#[inline(always)]
	fn one() -> Self
	{
		Self
		{ 
			x: T::one(), y: T::one() 
		}
	}
}

impl<T> UnitX for Vec2<T> where T:One + Zero
{
	#[inline(always)]
	fn unit_x() -> Self
	{
		Self
		{ 
			x: T::one(), y: T::zero() 
		}
	}
}

impl<T> UnitY for Vec2<T> where T:One + Zero
{
	#[inline(always)]
	fn unit_y() -> Self
	{
		Self
		{ 
			x: T::zero(), y: T::one()
		}
	}
}

impl<T> fmt::Display for Vec2<T> where T:Debug
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		write!(f, "({:?}, {:?})", self.x, self.y)
	}
}

impl<T> fmt::Binary for Vec2<T> where T:Vec + Math
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		let len = self.length();
		let decimals = f.precision().unwrap_or(3);
		let string = format!("{:.*?}", decimals, len);
		f.pad_integral(true, "", &string)
	}
}

impl<T> From<[T;2]> for Vec2<T> where T:Copy
{
	fn from(v:[T;2]) -> Self
	{
		Self
		{
			x:v[0],
			y:v[1]
		}
	}
}

impl<T> From<(T,T)> for Vec2<T> where T:Copy
{
	fn from(v:(T,T)) -> Self
	{
		Self
		{
			x:v.0,
			y:v.1,
		}
	}
}

impl<T> AsRef<Vec2<T>> for Vec2<T>
{
	fn as_ref(&self) -> &Vec2<T>
	{
		self
	}
}

impl<T> AsMut<Vec2<T>> for Vec2<T>
{
	fn as_mut(&mut self) -> &mut Vec2<T>
	{
		self
	}
}