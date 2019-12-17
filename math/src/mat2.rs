use std::ops::{Add, Sub, Mul};
use super::vec::{Vec, Math};
use super::vec2::Vec2;
use super::vec3::Vec3;
use super::consts::{Zero, One};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Mat2<T>
{
	pub a1:T, pub a2:T,
	pub b1:T, pub b2:T,
}

impl<T> Add for Mat2<T>  where T:Add<Output=T>
{
	type Output = Self;

	fn add(self, b: Self) -> Self
	{
		Mat2
		{
			a1:self.a1 + b.a1,
			a2:self.a2 + b.a2,
			b1:self.b1 + b.b1,
			b2:self.b2 + b.b2,
		}
	}
}

impl<T> Sub for Mat2<T>  where T:Sub<Output=T>
{
	type Output = Self;

	fn sub(self, b: Self) -> Self
	{
		Mat2
		{
			a1:self.a1 - b.a1,
			a2:self.a2 - b.a2,
			b1:self.b1 - b.b1,
			b2:self.b2 - b.b2,
		}
	}
}

impl<T> Mul for Mat2<T> where T:Copy + Mul<Output=T> + Add<Output=T>
{
	type Output = Self;

	fn mul(self, b: Self) -> Self
	{
		let a = self;

		Mat2
		{ 
			a1 : a.a1 * b.a1 + a.b1 * b.a2,
			a2 : a.a2 * b.a1 + a.b2 * b.a2,
			b1 : a.a1 * b.b1 + a.b1 * b.b2,
			b2 : a.a2 * b.b1 + a.b2 * b.b2
		}
	}
}

impl<T> Mat2<T> where T:Copy
{
	pub fn new(m11:T, m12:T, m21:T, m22:T) -> Self
	{
		Self
		{ 
			a1:m11, a2:m12,
			b1:m21, b2:m22,
		}
	}

	pub fn right(&self) -> Vec2<T>
	{
		Vec2::new(self.a1, self.a2)
	}

	pub fn up(&self) -> Vec2<T>
	{
		Vec2::new(self.b1, self.b2)
	}

	pub fn as_ptr(&self) -> *const T
	{
		&self.a1
	}

	pub fn to_array(&self) -> [T; 4]
	{
		[
			self.a1, self.a2,
			self.b1, self.b2,
		]
	}
}

impl<T> Mat2<T> where T:Vec + Math
{
	pub fn rotate_x(theta:T) -> Self
	{
		let (_s,c) = theta.sincos();
		Mat2::new(T::one(), T::zero(), T::zero(), c)
	}

	pub fn rotate_y(theta:T) -> Self
	{
		let (_s,c) = theta.sincos();
		Mat2::new(c, T::zero(), T::zero(), T::one())
	}

	pub fn rotate_z(theta:T) -> Self
	{
		let (s,c) = theta.sincos();
		Mat2::new(c, -s, s, c)
	}

	pub fn rotate(axis:Vec3<T>, theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let x = axis.x;
		let y = axis.y;
		let z = axis.z;

		let t = T::one() - c;
		let tx = t * x;
		let ty = t * y;

		let a1 = tx * x + c;
		let a2 = tx * y + s * z;

		let b1 = tx * y - s * z;
		let b2 = ty * y + c;

		Mat2
		{
			a1:a1, a2:a2,
			b1:b1, b2:b2
		}
	}

	pub fn scale(x:T, y:T) -> Self
	{
		Mat2
		{
			a1:x, a2:T::zero(),
			b1:T::zero(), b2:y
		}
	}

	pub fn transpose(&self) -> Self
	{
		let (a1, a2) = (self.a1, self.b1);
		let (b1, b2) = (self.a2, self.b2);

		Mat2
		{
			a1:a1, a2:a2,
			b1:b1, b2:b2
		}
	}
}

impl<T:Vec> Zero for Mat2<T>
{
	#[inline(always)]
	fn zero() -> Self
	{
		Mat2
		{
			a1:T::zero(), a2:T::zero(),
			b1:T::zero(), b2:T::zero()
		}
	}
}

impl<T:Vec> One for Mat2<T>
{
	#[inline(always)]
	fn one() -> Self
	{
		Mat2
		{
			a1:T::one(), a2:T::zero(),
			b1:T::zero(), b2:T::one()
		}
	}
}

impl<T> From<[T;4]> for Mat2<T> where T:Copy
{
	fn from(v:[T;4]) -> Self
	{
		Self
		{
			a1:v[0],a2:v[1],
			b1:v[2],b2:v[3],
		}
	}
}

impl<T> From<(T,T,T,T)> for Mat2<T> where T:Copy
{
	fn from(v:(T,T,T,T)) -> Self
	{
		Self
		{
			a1:v.0,
			a2:v.1,
			b1:v.2,
			b2:v.3,
		}
	}
}

impl<T> AsRef<Mat2<T>> for Mat2<T>
{
	fn as_ref(&self) -> &Mat2<T>
	{
		self
	}
}

impl<T> AsMut<Mat2<T>> for Mat2<T>
{
	fn as_mut(&mut self) -> &mut Mat2<T>
	{
		self
	}
}