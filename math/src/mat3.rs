use std::ops::{Add, Sub, Mul};
use super::vec::{Vec, Math};
use super::vec3::Vec3;
use super::quat::Quat;
use super::consts::{Zero, One};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Mat3<T>
{
	pub a1:T, pub a2:T, pub a3:T,
	pub b1:T, pub b2:T, pub b3:T,
	pub c1:T, pub c2:T, pub c3:T,
}

impl<T> Add for Mat3<T>  where T:Add<Output=T>
{
	type Output = Self;

	fn add(self, m: Self) -> Self
	{
		Mat3
		{
			a1:self.a1 + m.a1, a2:self.a2 + m.a2, a3:self.a3 + m.a3,
			b1:self.b1 + m.b1, b2:self.b2 + m.b2, b3:self.b3 + m.b3,
			c1:self.c1 + m.c1, c2:self.c2 + m.c2, c3:self.c3 + m.c3,
		}
	}
}

impl<T> Sub for Mat3<T>  where T:Sub<Output=T>
{
	type Output = Self;

	fn sub(self, m: Self) -> Self
	{
		Mat3
		{
			a1:self.a1 - m.a1, a2:self.a2 - m.a2, a3:self.a3 - m.a3,
			b1:self.b1 - m.b1, b2:self.b2 - m.b2, b3:self.b3 - m.b3,
			c1:self.c1 - m.c1, c2:self.c2 - m.c2, c3:self.c3 - m.c3,
		}
	}
}

impl<T> Mul for Mat3<T> where T:Copy + Mul<Output=T> + Add<Output=T>
{
	type Output = Self;

	fn mul(self, m: Self) -> Self
	{
		let a = self;

		Self
		{
			a1 : a.a1 * m.a1 + a.b1 * m.a2 + a.c1 * m.a3,
			a2 : a.a2 * m.a1 + a.b2 * m.a2 + a.c2 * m.a3,
			a3 : a.a3 * m.a1 + a.b3 * m.a2 + a.c3 * m.a3,

			b1 : a.a1 * m.b1 + a.b1 * m.b2 + a.c1 * m.b3,
			b2 : a.a2 * m.b1 + a.b2 * m.b2 + a.c2 * m.b3,
			b3 : a.a3 * m.b1 + a.b3 * m.b2 + a.c3 * m.b3,

			c1 : a.a1 * m.c1 + a.b1 * m.c2 + a.c1 * m.c3,
			c2 : a.a2 * m.c1 + a.b2 * m.c2 + a.c2 * m.c3,
			c3 : a.a3 * m.c1 + a.b3 * m.c2 + a.c3 * m.c3,
		}
	}
}

impl<T> Mat3<T> where T:Copy
{
	pub fn new(
		m11:T, m12:T, m13:T, 
		m21:T, m22:T, m23:T, 
		m31:T, m32:T, m33:T) -> Self
	{
		Self
		{ 
			a1:m11, a2:m12, a3:m13,
			b1:m21, b2:m22, b3:m23,
			c1:m31, c2:m32, c3:m33,
		}
	}

	pub fn right(&self) -> Vec3<T>
	{
		Vec3::new(self.a1, self.a2, self.a3)
	}

	pub fn up(&self) -> Vec3<T>
	{
		Vec3::new(self.b1, self.b2, self.b3)
	}

	pub fn forward(&self) -> Vec3<T>
	{
		Vec3::new(self.c1, self.c2, self.c3)
	}

	pub fn as_ptr(&self) -> *const T
	{
		&self.a1
	}

	pub fn to_array(&self) -> [T; 9]
	{
		[
			self.a1, self.a2, self.a3,
			self.b1, self.b2, self.b3,
			self.c1, self.c2, self.c3,
		]
	}
}

impl<T> Mat3<T> where T:Vec + Math
{
	pub fn rotate_x(theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let a1 = T::one();
		let a2 = T::zero();
		let a3 = T::zero();

		let b1 = T::zero();
		let b2 = c;
		let b3 = s;

		let c1 = T::zero();
		let c2 =-s;
		let c3 = c;

		Mat3::new(
			a1, a2, a3, 
			b1, b2, b3, 
			c1, c2, c3
		)
	}

	pub fn rotate_y(theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let a1 = c;
		let a2 = T::zero();
		let a3 =-s;

		let b1 = T::zero();
		let b2 = T::one();
		let b3 = T::zero();

		let c1 = s;
		let c2 = T::zero();
		let c3 = c;

		Mat3::new(
			a1, a2, a3, 
			b1, b2, b3, 
			c1, c2, c3
		)
	}

	pub fn rotate_z(theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let a1 = c;
		let a2 = s;
		let a3 = T::zero();

		let b1 =-s;
		let b2 = c;
		let b3 = T::zero();

		let c1 = T::zero();
		let c2 = T::zero();
		let c3 = T::one();

		Mat3::new(
			a1, a2, a3, 
			b1, b2, b3, 
			c1, c2, c3, 
		)
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
		let tz = t * z;

		let a1 = tx * x + c;
		let a2 = tx * y + s * z;
		let a3 = tx * z - s * y;

		let b1 = tx * y - s * z;
		let b2 = ty * y + c;
		let b3 = ty * z + s * x;

		let c1 = tx * z + s * y;
		let c2 = ty * z - s * x;
		let c3 = tz * z + c;

		Mat3::new(
			a1, a2, a3, 
			b1, b2, b3, 
			c1, c2, c3
		)
	}

	pub fn scale(x:T, y:T, z:T) -> Self
	{
		let (a1,a2,a3) = (x, T::zero(), T::zero());
		let (b1,b2,b3) = (T::zero(), y, T::zero());
		let (c1,c2,c3) = (T::zero(), T::zero(), z);

		Mat3::new(
			a1, a2, a3, 
			b1, b2, b3, 
			c1, c2, c3,
		)
	}

	pub fn translate(x:T, y:T) -> Self
	{
		let (a1,a2,a3) = (T::one(), T::zero(), T::zero());
		let (b1,b2,b3) = (T::zero(), T::one(), T::one());
		let (c1,c2,c3) = (x, y, T::one());

		Mat3::new(
			a1, a2, a3, 
			b1, b2, b3, 
			c1, c2, c3,
		)
	}
}

impl<T> Zero for Mat3<T> where T:Zero
{
	#[inline(always)]
	fn zero() -> Self
	{
		Self
		{
			a1:T::zero(), a2:T::zero(), a3:T::zero(),
			b1:T::zero(), b2:T::zero(), b3:T::zero(),
			c1:T::zero(), c2:T::zero(), c3:T::zero(),
		}
	}
}

impl<T> One for Mat3<T> where T:One + Zero
{
	#[inline(always)]
	fn one() -> Self
	{
		Self
		{
			a1:T::one(), a2:T::zero(), a3:T::zero(),
			b1:T::zero(), b2:T::one(), b3:T::zero(),
			c1:T::zero(), c2:T::zero(), c3:T::one(),
		}
	}
}

impl<T:Vec> From<Quat<T>> for Mat3<T>
{
	fn from(q:Quat<T>) -> Self
	{
		let (xs,ys,zs) = (q.x * T::two(), q.y * T::two(), q.z * T::two());
		
		let (xx,xy,xz) = (q.x * xs, q.x * ys, q.x * zs);
		let (yy,yz,zz) = (q.y * ys, q.y * zs, q.z * zs);
		let (wx,wy,wz) = (q.w * xs, q.w * ys, q.w * zs);

		Self
		{
			a1:T::one() - (yy + zz),
			a2:xy + wz,
			a3:xz - wy,

			b1:xy - wz,
			b2:T::one() - (xx + zz),
			b3:yz + wx,

			c1:xz + wy,
			c2:yz - wx,
			c3:T::one() - (xx + yy),
		}
	}
}

impl<T> From<[T;9]> for Mat3<T> where T:Copy
{
	fn from(v:[T;9]) -> Self
	{
		Self
		{
			a1:v[0],a2:v[1],a3:v[2],
			b1:v[3],b2:v[4],b3:v[5],
			c1:v[6],c2:v[7],c3:v[8],
		}
	}
}

impl<T> From<(T,T,T,T,T,T,T,T,T)> for Mat3<T> where T:Copy
{
	fn from(v:(T,T,T,T,T,T,T,T,T)) -> Self
	{
		Self
		{
			a1:v.0,a2:v.1,a3:v.2,
			b1:v.3,b2:v.4,b3:v.5,
			c1:v.6,c2:v.7,c3:v.8,
		}
	}
}

impl<T> AsRef<Mat3<T>> for Mat3<T>
{
	fn as_ref(&self) -> &Mat3<T>
	{
		self
	}
}

impl<T> AsMut<Mat3<T>> for Mat3<T>
{
	fn as_mut(&mut self) -> &mut Mat3<T>
	{
		self
	}
}