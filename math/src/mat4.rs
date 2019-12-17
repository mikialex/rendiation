use std::ops::{Add, Sub, Mul, Div};
use super::vec::{Vec, Math};
use super::vec3::Vec3;
use super::vec4::Vec4;
use super::quat::Quat;
use super::dual::Dual;
use super::mat3::Mat3;
use super::consts::{Zero, One, PiByC180};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Mat4<T>
{
	pub a1:T, pub a2:T, pub a3:T, pub a4:T,
	pub b1:T, pub b2:T, pub b3:T, pub b4:T,
	pub c1:T, pub c2:T, pub c3:T, pub c4:T,
	pub d1:T, pub d2:T, pub d3:T, pub d4:T,
}

impl<T> Add for Mat4<T>  where T:Add<Output=T>
{
	type Output = Mat4<T>;

	fn add(self, m:Self) -> Self
	{
		Mat4
		{
			a1:self.a1 + m.a1, a2:self.a2 + m.a2, a3:self.a3 + m.a3, a4:self.a4 + m.a4,
			b1:self.b1 + m.b1, b2:self.b2 + m.b2, b3:self.b3 + m.b3, b4:self.b4 + m.b4,
			c1:self.c1 + m.c1, c2:self.c2 + m.c2, c3:self.c3 + m.c3, c4:self.c4 + m.c4,
			d1:self.d1 + m.d1, d2:self.d2 + m.d2, d3:self.d3 + m.d3, d4:self.d4 + m.d4,
		}
	}
}

impl<T> Sub for Mat4<T>  where T:Sub<Output=T>
{
	type Output = Mat4<T>;

	fn sub(self, m:Self) -> Self
	{
		Mat4
		{
			a1:self.a1 - m.a1, a2:self.a2 - m.a2, a3:self.a3 - m.a3, a4:self.a4 - m.a4,
			b1:self.b1 - m.b1, b2:self.b2 - m.b2, b3:self.b3 - m.b3, b4:self.b4 - m.b4,
			c1:self.c1 - m.c1, c2:self.c2 - m.c2, c3:self.c3 - m.c3, c4:self.c4 - m.c4,
			d1:self.d1 - m.d1, d2:self.d2 - m.d2, d3:self.d3 - m.d3, d4:self.d4 - m.d4,
		}
	}
}

impl<T> Mul<Mat4<T>> for Vec3<T> where T:Copy + Add<Output=T> + Div<Output=T> + Mul<Output=T>
{
	type Output = Self;

	fn mul(self, m:Mat4<T>) -> Self
	{
		Self
		{
			x:(self.x * m.a1 + self.y * m.b1 + self.z * m.c1) / m.d4,
			y:(self.x * m.a2 + self.y * m.b2 + self.z * m.c2) / m.d4,
			z:(self.x * m.a3 + self.y * m.b3 + self.z * m.c3) / m.d4,
		}
	}
}

impl<T> Mul<Mat4<T>> for Vec4<T> where T:Copy + Add<Output=T> + Mul<Output=T>
{
	type Output = Self;

	fn mul(self, m:Mat4<T>) -> Self
	{
		Self
		{
			x:(self.x * m.a1 + self.y * m.b1 + self.z * m.c1 + self.w * m.d1),
			y:(self.x * m.a2 + self.y * m.b2 + self.z * m.c2 + self.w * m.d2),
			z:(self.x * m.a3 + self.y * m.b3 + self.z * m.c3 + self.w * m.d3),
			w:(self.x * m.a4 + self.y * m.b4 + self.z * m.c4 + self.w * m.d4),
		}
	}
}

impl<T> Mul<Vec3<T>> for Mat4<T> where T:Copy + Add<Output=T> + Sub<Output=T> + Mul<Output=T>
{
	type Output = Self;

	fn mul(self, v:Vec3<T>) -> Self
	{
		let a = self;

		Self
		{
			a1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1,
			a2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2,
			a3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3,
			a4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4,

			b1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1,
			b2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2,
			b3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3,
			b4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4,

			c1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1,
			c2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2,
			c3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3,
			c4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4,

			d1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1,
			d2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2,
			d3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3,
			d4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4,
		}
	}
}

impl<T> Mul<Vec4<T>> for Mat4<T> where T:Copy + Add<Output=T> + Mul<Output=T>
{
	type Output = Self;

	fn mul(self, v:Vec4<T>) -> Self
	{
		let a = self;

		Self
		{
			a1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1 * v.w,
			a2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2 * v.w,
			a3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3 * v.w,
			a4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4 * v.w,

			b1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1 * v.w,
			b2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2 * v.w,
			b3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3 * v.w,
			b4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4 * v.w,

			c1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1 * v.w,
			c2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2 * v.w,
			c3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3 * v.w,
			c4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4 * v.w,

			d1 : a.a1 * v.x + a.b1 * v.y + a.c1 * v.z + a.d1 * v.w,
			d2 : a.a2 * v.x + a.b2 * v.y + a.c2 * v.z + a.d2 * v.w,
			d3 : a.a3 * v.x + a.b3 * v.y + a.c3 * v.z + a.d3 * v.w,
			d4 : a.a4 * v.x + a.b4 * v.y + a.c4 * v.z + a.d4 * v.w,
		}
	}
}

impl<T> Mul for Mat4<T> where T:Copy + Mul<Output=T> + Add<Output=T>
{
	type Output = Mat4<T>;

	fn mul(self, m:Self) -> Self
	{
		let a = self;

		Self
		{
			a1 : a.a1 * m.a1 + a.b1 * m.a2 + a.c1 * m.a3 + a.d1 * m.a4,
			a2 : a.a2 * m.a1 + a.b2 * m.a2 + a.c2 * m.a3 + a.d2 * m.a4,
			a3 : a.a3 * m.a1 + a.b3 * m.a2 + a.c3 * m.a3 + a.d3 * m.a4,
			a4 : a.a4 * m.a1 + a.b4 * m.a2 + a.c4 * m.a3 + a.d4 * m.a4,

			b1 : a.a1 * m.b1 + a.b1 * m.b2 + a.c1 * m.b3 + a.d1 * m.b4,
			b2 : a.a2 * m.b1 + a.b2 * m.b2 + a.c2 * m.b3 + a.d2 * m.b4,
			b3 : a.a3 * m.b1 + a.b3 * m.b2 + a.c3 * m.b3 + a.d3 * m.b4,
			b4 : a.a4 * m.b1 + a.b4 * m.b2 + a.c4 * m.b3 + a.d4 * m.b4,

			c1 : a.a1 * m.c1 + a.b1 * m.c2 + a.c1 * m.c3 + a.d1 * m.c4,
			c2 : a.a2 * m.c1 + a.b2 * m.c2 + a.c2 * m.c3 + a.d2 * m.c4,
			c3 : a.a3 * m.c1 + a.b3 * m.c2 + a.c3 * m.c3 + a.d3 * m.c4,
			c4 : a.a4 * m.c1 + a.b4 * m.c2 + a.c4 * m.c3 + a.d4 * m.c4,

			d1 : a.a1 * m.d1 + a.b1 * m.d2 + a.c1 * m.d3 + a.d1 * m.d4,
			d2 : a.a2 * m.d1 + a.b2 * m.d2 + a.c2 * m.d3 + a.d2 * m.d4,
			d3 : a.a3 * m.d1 + a.b3 * m.d2 + a.c3 * m.d3 + a.d3 * m.d4,
			d4 : a.a4 * m.d1 + a.b4 * m.d2 + a.c4 * m.d3 + a.d4 * m.d4,
		}
	}
}

impl<T> Mat4<T> where T: Copy
{
	pub fn new(
		m11:T, m12:T, m13:T, m14:T, 
		m21:T, m22:T, m23:T, m24:T, 
		m31:T, m32:T, m33:T, m34:T, 
		m41:T, m42:T, m43:T, m44:T) -> Self
	{
		Self
		{ 
			a1:m11, a2:m12, a3:m13, a4:m14,
			b1:m21, b2:m22, b3:m23, b4:m24,
			c1:m31, c2:m32, c3:m33, c4:m34,
			d1:m41, d2:m42, d3:m43, d4:m44,
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

	pub fn position(&self) -> Vec3<T>
	{
		Vec3::new(self.d1, self.d2, self.d3)
	}
}

impl Mat4<f32> {
	pub fn max_scale_on_axis(&self) -> f32
	{
		let scale_x_sq = self.a1 * self.a1 + self.a2 * self.a2 + self.a3 * self.a3;
		let scale_y_sq = self.b1 * self.b1 + self.b2 * self.b2 + self.b3 * self.b3;
		let scale_z_sq = self.c1 * self.c1 + self.c2 * self.c2 + self.c3 * self.c3;

		scale_x_sq.max(scale_y_sq).max(scale_z_sq).sqrt()
	}
}

impl<T> Mat4<T> where T:Vec + Math + PiByC180
{
	pub fn rotate_x(theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let a1 = T::one();
		let a2 = T::zero();
		let a3 = T::zero();
		let a4 = T::zero();

		let b1 = T::zero();
		let b2 = c;
		let b3 = s;
		let b4 = T::zero();

		let c1 = T::zero();
		let c2 =-s;
		let c3 = c;
		let c4 = T::zero();

		let d1 = T::zero();
		let d2 = T::zero();
		let d3 = T::zero();
		let d4 = T::one();

		Mat4::new(
			a1, a2, a3, a4, 
			b1, b2, b3, b4, 
			c1, c2, c3, c4, 
			d1, d2, d3, d4
		)
	}

	pub fn rotate_y(theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let a1 = c;
		let a2 = T::zero();
		let a3 =-s;
		let a4 = T::zero();

		let b1 = T::zero();
		let b2 = T::one();
		let b3 = T::zero();
		let b4 = T::zero();

		let c1 = s;
		let c2 = T::zero();
		let c3 = c;
		let c4 = T::zero();

		let d1 = T::zero();
		let d2 = T::zero();
		let d3 = T::zero();
		let d4 = T::one();

		Mat4::new(
			a1, a2, a3, a4, 
			b1, b2, b3, b4, 
			c1, c2, c3, c4, 
			d1, d2, d3, d4
		)
	}

	pub fn rotate_z(theta:T) -> Self
	{
		let (s,c) = theta.sincos();

		let a1 = c;
		let a2 = s;
		let a3 = T::zero();
		let a4 = T::zero();

		let b1 =-s;
		let b2 = c;
		let b3 = T::zero();
		let b4 = T::zero();

		let c1 = T::zero();
		let c2 = T::zero();
		let c3 = T::one();
		let c4 = T::zero();

		let d1 = T::zero();
		let d2 = T::zero();
		let d3 = T::zero();
		let d4 = T::one();

		Mat4::new(
			a1, a2, a3, a4, 
			b1, b2, b3, b4, 
			c1, c2, c3, c4, 
			d1, d2, d3, d4
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
		let a4 = T::zero();

		let b1 = tx * y - s * z;
		let b2 = ty * y + c;
		let b3 = ty * z + s * x;
		let b4 = T::zero();

		let c1 = tx * z + s * y;
		let c2 = ty * z - s * x;
		let c3 = tz * z + c;
		let c4 = T::zero();

		let d1 = T::zero();
		let d2 = T::zero();
		let d3 = T::zero();		
		let d4 = T::one();

		Mat4::new(
			a1, a2, a3, a4, 
			b1, b2, b3, b4, 
			c1, c2, c3, c4, 
			d1, d2, d3, d4
		)
	}

	pub fn scale(x:T, y:T, z:T) -> Self
	{
		let (a1,a2,a3,a4) = (x, T::zero(), T::zero(), T::zero());
		let (b1,b2,b3,b4) = (T::zero(), y, T::zero(), T::zero());
		let (c1,c2,c3,c4) = (T::zero(), T::zero(), z, T::zero());
		let (d1,d2,d3,d4) = (T::zero(), T::zero(), T::zero(), T::one());

		Mat4::new(
			a1, a2, a3, a4, 
			b1, b2, b3, b4, 
			c1, c2, c3, c4, 
			d1, d2, d3, d4
		)
	}

	pub fn translate(x:T, y:T, z:T) -> Self
	{
		let (a1,a2,a3,a4) = (T::one(), T::zero(), T::zero(), T::zero());
		let (b1,b2,b3,b4) = (T::zero(), T::one(), T::zero(), T::zero());
		let (c1,c2,c3,c4) = (T::zero(), T::zero(), T::one(), T::zero());
		let (d1,d2,d3,d4) = (x, y, z, T::one());

		Mat4::new(
			a1, a2, a3, a4, 
			b1, b2, b3, b4, 
			c1, c2, c3, c4, 
			d1, d2, d3, d4
		)
	}

	pub fn det(&self) -> T
	{
		let m = self;
		m.a1*m.b2*m.c3*m.d4 - m.a1*m.b2*m.c4*m.d3 + m.a1*m.b3*m.c4*m.d2 - m.a1*m.b3*m.c2*m.d4 +
		m.a1*m.b4*m.c2*m.d3 - m.a1*m.b4*m.c3*m.d2 - m.a2*m.b3*m.c4*m.d1 + m.a2*m.b3*m.c1*m.d4 -
		m.a2*m.b4*m.c1*m.d3 + m.a2*m.b4*m.c3*m.d1 - m.a2*m.b1*m.c3*m.d4 + m.a2*m.b1*m.c4*m.d3 +
		m.a3*m.b4*m.c1*m.d2 - m.a3*m.b4*m.c2*m.d1 + m.a3*m.b1*m.c2*m.d4 - m.a3*m.b1*m.c4*m.d2 +
		m.a3*m.b2*m.c4*m.d1 - m.a3*m.b2*m.c1*m.d4 - m.a4*m.b1*m.c2*m.d3 + m.a4*m.b1*m.c3*m.d2 -
		m.a4*m.b2*m.c3*m.d1 + m.a4*m.b2*m.c1*m.d3 - m.a4*m.b3*m.c1*m.d2 + m.a4*m.b3*m.c2*m.d1
	}

	pub fn inverse(&self) -> Self
	{
		let det = self.det();
		if det.eq(T::zero())
		{
			return Mat4::one();
		}

		let m = self;
		let invdet = T::one() / det;

		Self
		{
			a1: invdet * (m.b2 * (m.c3 * m.d4 - m.c4 * m.d3) + m.b3 * (m.c4 * m.d2 - m.c2 * m.d4) + m.b4 * (m.c2 * m.d3 - m.c3 * m.d2)),
			a2:-invdet * (m.a2 * (m.c3 * m.d4 - m.c4 * m.d3) + m.a3 * (m.c4 * m.d2 - m.c2 * m.d4) + m.a4 * (m.c2 * m.d3 - m.c3 * m.d2)),
			a3: invdet * (m.a2 * (m.b3 * m.d4 - m.b4 * m.d3) + m.a3 * (m.b4 * m.d2 - m.b2 * m.d4) + m.a4 * (m.b2 * m.d3 - m.b3 * m.d2)),
			a4:-invdet * (m.a2 * (m.b3 * m.c4 - m.b4 * m.c3) + m.a3 * (m.b4 * m.c2 - m.b2 * m.c4) + m.a4 * (m.b2 * m.c3 - m.b3 * m.c2)),
			b1:-invdet * (m.b1 * (m.c3 * m.d4 - m.c4 * m.d3) + m.b3 * (m.c4 * m.d1 - m.c1 * m.d4) + m.b4 * (m.c1 * m.d3 - m.c3 * m.d1)),
			b2: invdet * (m.a1 * (m.c3 * m.d4 - m.c4 * m.d3) + m.a3 * (m.c4 * m.d1 - m.c1 * m.d4) + m.a4 * (m.c1 * m.d3 - m.c3 * m.d1)),
			b3:-invdet * (m.a1 * (m.b3 * m.d4 - m.b4 * m.d3) + m.a3 * (m.b4 * m.d1 - m.b1 * m.d4) + m.a4 * (m.b1 * m.d3 - m.b3 * m.d1)),
			b4: invdet * (m.a1 * (m.b3 * m.c4 - m.b4 * m.c3) + m.a3 * (m.b4 * m.c1 - m.b1 * m.c4) + m.a4 * (m.b1 * m.c3 - m.b3 * m.c1)),
			c1: invdet * (m.b1 * (m.c2 * m.d4 - m.c4 * m.d2) + m.b2 * (m.c4 * m.d1 - m.c1 * m.d4) + m.b4 * (m.c1 * m.d2 - m.c2 * m.d1)),
			c2:-invdet * (m.a1 * (m.c2 * m.d4 - m.c4 * m.d2) + m.a2 * (m.c4 * m.d1 - m.c1 * m.d4) + m.a4 * (m.c1 * m.d2 - m.c2 * m.d1)),
			c3: invdet * (m.a1 * (m.b2 * m.d4 - m.b4 * m.d2) + m.a2 * (m.b4 * m.d1 - m.b1 * m.d4) + m.a4 * (m.b1 * m.d2 - m.b2 * m.d1)),
			c4:-invdet * (m.a1 * (m.b2 * m.c4 - m.b4 * m.c2) + m.a2 * (m.b4 * m.c1 - m.b1 * m.c4) + m.a4 * (m.b1 * m.c2 - m.b2 * m.c1)),
			d1:-invdet * (m.b1 * (m.c2 * m.d3 - m.c3 * m.d2) + m.b2 * (m.c3 * m.d1 - m.c1 * m.d3) + m.b3 * (m.c1 * m.d2 - m.c2 * m.d1)),
			d2: invdet * (m.a1 * (m.c2 * m.d3 - m.c3 * m.d2) + m.a2 * (m.c3 * m.d1 - m.c1 * m.d3) + m.a3 * (m.c1 * m.d2 - m.c2 * m.d1)),
			d3:-invdet * (m.a1 * (m.b2 * m.d3 - m.b3 * m.d2) + m.a2 * (m.b3 * m.d1 - m.b1 * m.d3) + m.a3 * (m.b1 * m.d2 - m.b2 * m.d1)),
			d4: invdet * (m.a1 * (m.b2 * m.c3 - m.b3 * m.c2) + m.a2 * (m.b3 * m.c1 - m.b1 * m.c3) + m.a3 * (m.b1 * m.c2 - m.b2 * m.c1)),
		}
	}

	pub fn transform_inverse(&self) -> Self
	{
		let m = self;
		let det = (m.a1 * m.b2 - m.a2 * m.b1) * (m.c3) - (m.a1 * m.b3 - m.a3 * m.b1) * (m.c2) + (m.a2 * m.b3 - m.a3 * m.b2) * (m.c1);
		if det.eq(T::zero())
		{
			return Mat4::one();
		}

		let invdet = T::one() / det;

		Self
		{
			a1 : invdet * (m.b2 * m.c3 + m.b3 * -m.c2),
			a2 : invdet * (m.c2 * m.a3 + m.c3 * -m.a2),
			a3 : invdet * (m.a2 * m.b3 - m.a3 * m.b2),
			a4 : T::zero(),
			b1 : invdet * (m.b3 * m.c1 + m.b1 * -m.c3),
			b2 : invdet * (m.c3 * m.a1 + m.c1 * -m.a3),
			b3 : invdet * (m.a3 * m.b1 - m.a1 * m.b3),
			b4 : T::zero(),
			c1 : invdet * (m.b1 * m.c2 + m.b2 * -m.c1),
			c2 : invdet * (m.c1 * m.a2 + m.c2 * -m.a1),
			c3 : invdet * (m.a1 * m.b2 - m.a2 * m.b1),
			c4 : T::zero(),
			d1 : invdet * (m.b1 * (m.c3 * m.d2 - m.c2 * m.d3) + m.b2 * (m.c1 * m.d3 - m.c3 * m.d1) + m.b3 * (m.c2 * m.d1 - m.c1 * m.d2)),
			d2 : invdet * (m.c1 * (m.a3 * m.d2 - m.a2 * m.d3) + m.c2 * (m.a1 * m.d3 - m.a3 * m.d1) + m.c3 * (m.a2 * m.d1 - m.a1 * m.d2)),
			d3 : invdet * (m.d1 * (m.a3 * m.b2 - m.a2 * m.b3) + m.d2 * (m.a1 * m.b3 - m.a3 * m.b1) + m.d3 * (m.a2 * m.b1 - m.a1 * m.b2)),
			d4 : invdet * (m.a1 * (m.b2 * m.c3 - m.b3 * m.c2) + m.a2 * (m.b3 * m.c1 - m.b1 * m.c3) + m.a3 * (m.b1 * m.c2 - m.b2 * m.c1)),
		}
	}

	pub fn ortho_lh(left:T, right:T, bottom:T, top:T, znear:T, zfar:T) -> Self
	{
		let tx = -(right + left) / (right - left);
		let ty = -(top + bottom) / (top - bottom);
		let tz = -znear / (zfar - znear);
		let cx = T::two() / (right - left);
		let cy = T::two() / (top - bottom);
		let cz = T::one() / (zfar - znear);

		Mat4::new(
			cx, T::zero(), T::zero(), T::zero(),
			T::zero(), cy, T::zero(), T::zero(),
			T::zero(), T::zero(), cz, T::zero(),
			tx, ty, tz, T::one())
	}

	pub fn ortho_rh(left:T, right:T, bottom:T, top:T, znear:T, zfar:T) -> Self
	{
		let tx = -(right + left) / (right - left);
		let ty = -(top + bottom) / (top - bottom);
		let tz = -(zfar + znear) / (zfar - znear);
		let cx = T::two() / (right - left);
		let cy = T::two() / (top - bottom);
		let cz = -T::one() / (zfar - znear);

		Mat4::new(
			cx, T::zero(), T::zero(), T::zero(),
			T::zero(), cy, T::zero(), T::zero(),
			T::zero(), T::zero(), cz, T::zero(),
			tx, ty, tz, T::one())
	}

	pub fn perspective_fov_lh(fov:T, aspect:T, znear:T, zfar:T) -> Self
	{
		let h = aspect / (fov * T::onehalf() * T::pi_by_c180()).tan();
		let w = h / aspect;
		let q = zfar / (zfar - znear);

		Mat4::new(
			w, T::zero(), T::zero(), T::zero(),
			T::zero(), h, T::zero(), T::zero(),
			T::zero(), T::zero(), q, T::one(),
			T::zero(), T::zero(), -znear * q, T::zero()
			)
	}

	pub fn perspective_fov_rh(fov:T, aspect:T, znear:T, zfar:T) -> Self
	{
		let h = aspect / (fov * T::onehalf() * T::pi_by_c180()).tan();
		let w = h / aspect;
		let q = -zfar / (zfar - znear);

		Mat4::new(
			w, T::zero(), T::zero(), T::zero(),
			T::zero(), h, T::zero(), T::zero(),
			T::zero(), T::zero(), q, -T::one(),
			T::zero(), T::zero(), znear * q, T::zero()
			)
	}

	pub fn lookat_lh(eye:Vec3<T>, center:Vec3<T>, up:Vec3<T>) -> Self
	{
		let mut z = center - eye;
		z = z.normalize();

		let mut x = up.cross(z);
		x = x.normalize();

		let mut y = z.cross(x);
		y = y.normalize();

		let mut m = Mat4::new(
			x.x, y.x, z.x, T::zero(),
			x.y, y.y, z.y, T::zero(),
			x.z, y.z, z.z, T::zero(),
			T::zero(), T::zero(), T::zero(), T::one());

		let tmp = -eye;
		if tmp.x.ne(T::zero())
		{
			m.d1 += tmp.x * m.a1;
			m.d2 += tmp.x * m.a2;
			m.d3 += tmp.x * m.a3;
			m.d4 += tmp.x * m.a4;
		}

		if tmp.y.ne(T::zero())
		{
			m.d1 += tmp.y * m.b1;
			m.d2 += tmp.y * m.b2;
			m.d3 += tmp.y * m.b3;
			m.d4 += tmp.y * m.b4;
		}

		if tmp.z.ne(T::zero())
		{
			m.d1 += tmp.z * m.c1;
			m.d2 += tmp.z * m.c2;
			m.d3 += tmp.z * m.c3;
			m.d4 += tmp.z * m.c4;
		}

		return m;
	}

	pub fn lookat_rh(eye:Vec3<T>, center:Vec3<T>, up:Vec3<T>) -> Self
	{
		let mut z = eye - center;
		z = z.normalize();

		let mut x = up.cross(z);
		x = x.normalize();

		let mut y = z.cross(x);
		y = y.normalize();

		let mut m = Mat4::new(
			x.x, y.x, z.x, T::zero(),
			x.y, y.y, z.y, T::zero(),
			x.z, y.z, z.z, T::zero(),
			T::zero(), T::zero(), T::zero(), T::one());

		let tmp = -eye;
		if tmp.x.ne(T::zero())
		{
			m.d1 += tmp.x * m.a1;
			m.d2 += tmp.x * m.a2;
			m.d3 += tmp.x * m.a3;
			m.d4 += tmp.x * m.a4;
		}

		if tmp.y.ne(T::zero())
		{
			m.d1 += tmp.y * m.b1;
			m.d2 += tmp.y * m.b2;
			m.d3 += tmp.y * m.b3;
			m.d4 += tmp.y * m.b4;
		}

		if tmp.z.ne(T::zero())
		{
			m.d1 += tmp.z * m.c1;
			m.d2 += tmp.z * m.c2;
			m.d3 += tmp.z * m.c3;
			m.d4 += tmp.z * m.c4;
		}

		return m;
	}

	pub fn as_ptr(&self) -> *const T
	{
		&self.a1
	}

	pub fn to_array(&self) -> [T; 16]
	{
		[
			self.a1, self.a2, self.a3, self.a4,
			self.b1, self.b2, self.b3, self.b4,
			self.c1, self.c2, self.c3, self.c4,
			self.d1, self.d2, self.d3, self.d4,
		]
	}

	pub fn to_array_transpose(&self) -> [T; 16]
	{
		[
			self.a1, self.b1, self.c1, self.d1,
			self.a2, self.b2, self.c2, self.d2,
			self.a3, self.b3, self.c3, self.d3,
			self.a4, self.b4, self.c4, self.d4,
		]
	}

	pub fn transpose(&self) -> Mat4<T>
	{
		Mat4::new(
			self.a1, self.b1, self.c1, self.d1,
			self.a2, self.b2, self.c2, self.d2,
			self.a3, self.b3, self.c3, self.d3,
			self.a4, self.b4, self.c4, self.d4,
		)
	}
}

impl<T> Zero for Mat4<T> where T:Zero
{
	#[inline(always)]
	fn zero() -> Self
	{
		Self
		{ 
			a1:T::zero(), a2:T::zero(), a3:T::zero(), a4:T::zero(),
			b1:T::zero(), b2:T::zero(), b3:T::zero(), b4:T::zero(),
			c1:T::zero(), c2:T::zero(), c3:T::zero(), c4:T::zero(),
			d1:T::zero(), d2:T::zero(), d3:T::zero(), d4:T::zero(),
		}
	}
}

impl<T> One for Mat4<T> where T:One + Zero
{
	#[inline(always)]
	fn one() -> Self
	{
		Self
		{ 
			a1:T::one(), a2:T::zero(), a3:T::zero(), a4:T::zero(),
			b1:T::zero(), b2:T::one(), b3:T::zero(), b4:T::zero(),
			c1:T::zero(), c2:T::zero(), c3:T::one(), c4:T::zero(),
			d1:T::zero(), d2:T::zero(), d3:T::zero(), d4:T::one(),
		}
	}
}

impl<T:Vec> From<Mat3<T>> for Mat4<T>
{
	fn from(m:Mat3<T>) -> Self
	{
		Self
		{ 
			a1:m.a1, a2:m.a2, a3:m.a3, a4:T::zero(),
			b1:m.b1, b2:m.b2, b3:m.b3, b4:T::zero(),
			c1:m.c1, c2:m.c2, c3:m.c3, c4:T::zero(),
			d1:T::zero(), d2:T::zero(), d3:T::zero(), d4:T::one(),
		}
	}
}

impl<T:Vec> From<Quat<T>> for Mat4<T>
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
			a4:T::zero(),

			b1:xy - wz,
			b2:T::one() - (xx + zz),
			b3:yz + wx,
			b4:T::zero(),

			c1:xz + wy,
			c2:yz - wx,
			c3:T::one() - (xx + yy),
			c4:T::zero(),

			d1:T::zero(),
			d2:T::zero(),
			d3:T::zero(),
			d4:T::one()
		}
	}
}

impl<T:Vec> From<Dual<T>> for Mat4<T> where T:Math
{
	fn from(dual:Dual<T>) -> Self
	{
		let q = dual.real;

		let (xs,ys,zs) = (q.x * T::two(), q.y * T::two(), q.z * T::two());
		
		let (xx,xy,xz) = (q.x * xs, q.x * ys, q.x * zs);
		let (yy,yz,zz) = (q.y * ys, q.y * zs, q.z * zs);
		let (wx,wy,wz) = (q.w * xs, q.w * ys, q.w * zs);

		let t = dual.translate();

		Self
		{
			a1:T::one() - (yy + zz),
			a2:xy + wz,
			a3:xz - wy,
			a4:T::zero(),

			b1:xy - wz,
			b2:T::one() - (xx + zz),
			b3:yz + wx,
			b4:T::zero(),

			c1:xz + wy,
			c2:yz - wx,
			c3:T::one() - (xx + yy),
			c4:T::zero(),

			d1:t.x,
			d2:t.y,
			d3:t.z,
			d4:T::one()
		}
	}
}

impl<T> From<[T;16]> for Mat4<T> where T:Copy
{
	fn from(v:[T;16]) -> Self
	{
		Self
		{
			a1:v[0],a2:v[1],a3:v[2],a4:v[3],
			b1:v[4],b2:v[5],b3:v[6],b4:v[7],
			c1:v[8],c2:v[9],c3:v[10],c4:v[11],
			d1:v[12],d2:v[13],d3:v[14],d4:v[15],
		}
	}
}

impl<T> From<(T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T)> for Mat4<T> where T:Copy
{
	fn from(v:(T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T)) -> Self
	{
		Self
		{
			a1:v.0,a2:v.1,a3:v.2,a4:v.3,
			b1:v.4,b2:v.5,b3:v.6,b4:v.7,
			c1:v.8,c2:v.9,c3:v.10,c4:v.11,
			d1:v.12,d2:v.13,d3:v.14,d4:v.15,
		}
	}
}

impl<T> AsRef<Mat4<T>> for Mat4<T>
{
	fn as_ref(&self) -> &Mat4<T>
	{
		self
	}
}

impl<T> AsMut<Mat4<T>> for Mat4<T>
{
	fn as_mut(&mut self) -> &mut Mat4<T>
	{
		self
	}
}