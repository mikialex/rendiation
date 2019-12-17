use std::fmt;
use std::fmt::Debug;
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use super::vec::{Vec, Math, Lerp, Slerp};
use super::vec3::Vec3;
use super::vec4::Vec4;
use super::consts::{Zero, One};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Quat<T>
{
	pub x:T,
	pub y:T,
	pub z:T,
	pub w:T,
}

impl<T> Quat<T> {
	pub fn set(&mut self, x: T, y: T, z: T, w: T) -> &Self {
		self.x = x;
		self.y = y;
		self.z = z;
		self.w = w;
		self
	}
}

impl<T> Neg for Quat<T> where T:Neg<Output=T>
{
	type Output = Self;

	fn neg(self) -> Self
	{
		Self
		{
			x: -self.x,
			y: -self.y,
			z: -self.z,
			w: -self.w
		}
	}
}

impl<T> Add for Quat<T>  where T:Add<Output=T>
{
	type Output = Self;

	fn add(self, other: Self) -> Self
	{
		Self
		{
			x: self.x + other.x, 
			y: self.y + other.y, 
			z: self.z + other.z,
			w: self.w + other.w,
		}
	}
}

impl<T> Sub for Quat<T> where T:Sub<Output=T>
{
	type Output = Self;

	fn sub(self, other: Self) -> Self
	{
		Self
		{
			x: self.x - other.x, 
			y: self.y - other.y, 
			z: self.z - other.z,
			w: self.w - other.w,
		}
	}
}

impl<T> Mul<T> for Quat<T> where T:Mul<Output=T> + Copy
{
	type Output = Self;

	fn mul(self, s: T) -> Self
	{
		Self
		{
			x:self.x * s,
			y:self.y * s,
			z:self.z * s,
			w:self.w * s,
		}
	}
}

impl<T> Mul for Quat<T> where T:Mul<Output=T>
{
	type Output = Self;

	fn mul(self, other: Self) -> Self
	{
		Self
		{
			x: self.x * other.x, 
			y: self.y * other.y, 
			z: self.z * other.z,
			w: self.w * other.w,
		}
	}
}

impl<T> Div<T> for Quat<T> where T:Div<Output=T> + Copy
{
	type Output = Self;

	fn div(self, s: T) -> Self
	{
		Self
		{
			x:self.x / s,
			y:self.y / s,
			z:self.z / s,
			w:self.w / s,
		}
	}
}

impl<T> Div for Quat<T> where T:Div<Output=T>
{
	type Output = Self;

	fn div(self, other: Self) -> Self
	{
		Self
		{
			x: self.x / other.x, 
			y: self.y / other.y, 
			z: self.z / other.z,
			w: self.w / other.w,
		}
	}
}

impl<T> AddAssign for Quat<T> where T: AddAssign<T>
{
	fn add_assign(&mut self, other: Self)
	{
		self.x += other.x;
		self.y += other.y; 
		self.z += other.z; 
		self.w += other.w; 
	}
}

impl<T> SubAssign for Quat<T> where T: SubAssign<T>
{
	fn sub_assign(&mut self, other: Self)
	{
		self.x -= other.x;
		self.y -= other.y; 
		self.z -= other.z;
		self.w -= other.w;
	}
}

impl<T> MulAssign<T> for Quat<T> where T: MulAssign<T> + Copy
{
	fn mul_assign(&mut self, s: T)
	{
		self.x *= s;
		self.y *= s;
		self.z *= s;
		self.w *= s;
	}
}

impl<T> MulAssign for Quat<T> where T: MulAssign<T>
{
	fn mul_assign(&mut self, other: Self)
	{
		self.x *= other.x;
		self.y *= other.y; 
		self.z *= other.z;
		self.w *= other.w;
	}
}

impl<'a, T> MulAssign<&'a T> for Quat<T> where T:MulAssign<T> + Copy
{
	fn mul_assign(&mut self, other: &'a T)
	{
		self.x *= *other;
		self.y *= *other;
		self.z *= *other;
		self.w *= *other;
	}
}

impl<T> DivAssign<T> for Quat<T> where T: DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: T)
	{
		self.x /= s;
		self.y /= s;
		self.z /= s;
		self.w /= s;
	}
}

impl<T> DivAssign for Quat<T> where T: DivAssign<T>
{
	fn div_assign(&mut self, other: Self)
	{
		self.x /= other.x;
		self.y /= other.y; 
		self.z /= other.z;
		self.w /= other.w;
	}
}

impl<'a, T> DivAssign<&'a T> for Quat<T> where T:DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: &'a T)
	{
		self.x /= *s;
		self.y /= *s;
		self.z /= *s;
		self.w /= *s;
	}
}

impl<T> Quat<T> where T:Copy
{
	/// Creates a new Quat from multiple components
	pub fn new(x: T, y: T, z: T, w: T) -> Self { Self { x, y, z, w } }

	pub fn len() -> usize 
	{
		return 4;
	}

	pub fn to_tuple(&self) -> (T, T, T, T)
	{
		(self.x, self.y, self.z, self.w)
	}
}

impl<T> Quat<T> where T:Vec + Math
{
	pub fn rotation_x(theta:T) -> Self
	{
		let theta_half = theta * T::onehalf();

		Self
		{
			w:theta_half.cos(),
			x:theta_half.sin(),
			y:T::zero(),
			z:T::zero(),
		}
	}

	pub fn rotation_y(theta:T) -> Self
	{
		let theta_half = theta * T::onehalf();

		Self
		{
			w:theta_half.cos(),
			x:T::zero(),
			y:theta_half.sin(),
			z:T::zero(),
		}
	}

	pub fn rotation_z(theta:T) -> Self
	{
		let theta_half = theta * T::onehalf();

		Self
		{
			w:theta_half.cos(),
			x:T::zero(),
			y:T::zero(),
			z:theta_half.sin(),
		}
	}

	pub fn rotation(axis:&Vec3<T>, theta:T) -> Self
	{
		let (s, c) = (theta * T::onehalf()).sincos();

		Self
		{
			w:c,
			x:axis.x * s,
			y:axis.y * s,
			z:axis.z * s,
		}
	}

	pub fn direction(a:&Vec3<T>, b:&Vec3<T>) -> Self
	{
		let axis = a.cross(*b);
		let cos_angle = a.dot(*b);

		let t0 = T::one() + cos_angle;
		let t1 = (t0 + t0).rsqrt();
		let t2 = (t0 + t0) * t1 * T::onehalf();

		Self
		{
			x:axis.x * t1,
			y:axis.y * t1,
			z:axis.z * t1,
			w:t2,
		}
	}

	pub fn euler_xyz(euler:&Vec3<T>) -> Self
	{
		let p = (euler.x * T::onehalf()).sincos();
		let h = (euler.y * T::onehalf()).sincos();
		let b = (euler.z * T::onehalf()).sincos();

		let sp = p.0; let sb = b.0; let sh = h.0;
		let cp = p.1; let cb = b.1; let ch = h.1;

		Self
		{
			w:cp * ch * cb + sp * sh * sb,
			x:sp * ch * cb - cp * sh * sb,
			y:cp * sh * cb + sp * ch * sb,
			z:cp * ch * sb - sp * sh * cb,
		}
	}

	pub fn euler_zxy(euler:&Vec3<T>) -> Self
	{
		let p = (euler.x * T::onehalf()).sincos();
		let h = (euler.y * T::onehalf()).sincos();
		let b = (euler.z * T::onehalf()).sincos();

		let sp = p.0; let sb = b.0; let sh = h.0;
		let cp = p.1; let cb = b.1; let ch = h.1;

		Self
		{
			w:cp * ch * cb + sp * sh * sb,
			x:cp * sh * cb + sp * ch * sb,
			y:cp * ch * sb - sp * sh * cb,
			z:sp * ch * cb - cp * sh * sb,
		}
	}

	pub fn dot(&self, b: Self) -> T 
	{
		return self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w;
	}
	
	pub fn cross(&self, b: Self) -> Self 
	{
		Self
		{
			x:self.w * b.x + self.x * b.w + self.z * b.y - self.y * b.z,
			y:self.w * b.y + self.y * b.w + self.x * b.z - self.z * b.x,
			z:self.w * b.z + self.z * b.w + self.y * b.x - self.x * b.y,
			w:self.w * b.w - self.x * b.x - self.y * b.y - self.z * b.z,
		}
	}
	
	pub fn length2(&self) -> T 
	{
		return self.dot(*self);
	}
	
	pub fn length(&self) -> T 
	{
		return self.length2().sqrt();
	}
	
	pub fn distance(&self, b: Self) -> T 
	{
		return (*self - b).length();
	}

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

	pub fn axis(&self) -> Vec3<T>
	{
		let sin_theta_over2_sq = T::one() - self.w * self.w;
		if sin_theta_over2_sq.le(T::zero())
		{
			return Vec3::new(T::one(), T::zero(), T::zero());
		}

		let v = Vec3::new(self.x, self.y, self.z);
		let inv_sqrt = T::one() / sin_theta_over2_sq.sqrt();

		return v * Vec3::new(inv_sqrt, inv_sqrt, inv_sqrt);
	}

	pub fn angle(&self) -> T
	{
		self.w.acos() * T::two()
	}

	pub fn conj(&self) -> Self
	{
		Self
		{ 
			x: -self.x, 
			y: -self.y,
			z: -self.z,
			w:  self.w
		}
	}

	pub fn conjugate(&self) -> Self
	{
		Self
		{ 
			x: -self.x, 
			y: -self.y,
			z: -self.z,
			w:  self.w
		}
	}

	pub fn inverse(&self) -> Self
	{
		self.conjugate()
	}
}

impl<T> Math for Quat<T> where T:Copy + Math
{
	fn abs(self) -> Self
	{
		let mx = self.x.abs();
		let my = self.y.abs();
		let mz = self.z.abs();
		let mw = self.w.abs();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn recip(self) -> Self
	{
		let mx = self.x.recip();
		let my = self.y.recip();
		let mz = self.z.recip();
		let mw = self.w.recip();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn sqrt(self) -> Self
	{
		let mx = self.x.sqrt();
		let my = self.y.sqrt();
		let mz = self.z.sqrt();
		let mw = self.w.sqrt();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn rsqrt(self) -> Self
	{
		let mx = self.x.rsqrt();
		let my = self.y.rsqrt();
		let mz = self.z.rsqrt();
		let mw = self.w.rsqrt();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn sin(self) -> Self
	{
		let mx = self.x.sin();
		let my = self.y.sin();
		let mz = self.z.sin();
		let mw = self.w.sin();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn cos(self) -> Self
	{
		let mx = self.x.cos();
		let my = self.y.cos();
		let mz = self.z.cos();
		let mw = self.w.cos();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn tan(self) -> Self
	{
		let mx = self.x.tan();
		let my = self.y.tan();
		let mz = self.z.tan();
		let mw = self.w.tan();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn sincos(self) -> (Quat<T>, Quat<T>)
	{
		let mx = self.x.sincos();
		let my = self.y.sincos();
		let mz = self.z.sincos();
		let mw = self.w.sincos();
		(
			Self { x: mx.0, y: my.0, z: mz.0, w: mw.0 },
			Self { x: mx.1, y: my.1, z: mz.1, w: mw.1 }
		)
	}

	fn acos(self) -> Self
	{
		let mx = self.x.acos();
		let my = self.y.acos();
		let mz = self.z.acos();
		let mw = self.w.acos();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn asin(self) -> Self
	{
		let mx = self.x.asin();
		let my = self.y.asin();
		let mz = self.z.asin();
		let mw = self.w.asin();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn atan(self) -> Self
	{
		let mx = self.x.atan();
		let my = self.y.atan();
		let mz = self.z.atan();
		let mw = self.w.atan();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn exp(self) -> Self
	{
		let mx = self.x.exp();
		let my = self.y.exp();
		let mz = self.z.exp();
		let mw = self.w.exp();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn exp2(self) -> Self
	{
		let mx = self.x.exp2();
		let my = self.y.exp2();
		let mz = self.z.exp2();
		let mw = self.w.exp2();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn log(self, _rhs: Self) -> Self
	{
		let mx = self.x.log(_rhs.x);
		let my = self.y.log(_rhs.y);
		let mz = self.z.log(_rhs.z);
		let mw = self.w.log(_rhs.w);
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn log2(self) -> Self
	{
		let mx = self.x.log2();
		let my = self.y.log2();
		let mz = self.z.log2();
		let mw = self.w.log2();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn log10(self) -> Self
	{
		let mx = self.x.log10();
		let my = self.y.log10();
		let mz = self.z.log10();
		let mw = self.w.log10();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn to_radians(self) -> Self
	{
		let mx = self.x.to_radians();
		let my = self.y.to_radians();
		let mz = self.z.to_radians();
		let mw = self.w.to_radians();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn to_degrees(self) -> Self
	{
		let mx = self.x.to_degrees();
		let my = self.y.to_degrees();
		let mz = self.z.to_degrees();
		let mw = self.w.to_degrees();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn min(self, _rhs: Self) -> Self
	{
		let mx = self.x.min(_rhs.x);
		let my = self.y.min(_rhs.y);
		let mz = self.z.min(_rhs.z);
		let mw = self.w.min(_rhs.x);
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn max(self, _rhs: Self) -> Self
	{
		let mx = self.x.max(_rhs.x);
		let my = self.y.max(_rhs.y);
		let mz = self.z.max(_rhs.z);
		let mw = self.w.max(_rhs.w);
		Self { x: mx, y: my, z: mz, w:mw }
	}

	fn saturate(self) -> Self
	{
		let mx = self.x.saturate();
		let my = self.y.saturate();
		let mz = self.z.saturate();
		let mw = self.w.saturate();
		Self { x: mx, y: my, z: mz, w: mw }
	}

	#[inline]
	fn snorm2unorm(self) -> Self
	{
		let mx = self.x.snorm2unorm();
		let my = self.y.snorm2unorm();
		let mz = self.z.snorm2unorm();
		let mw = self.w.snorm2unorm();
		Self { x: mx, y: my, z: mz, w: mw }
	}

	#[inline]
	fn unorm2snorm(self) -> Self
	{
		let mx = self.x.unorm2snorm();
		let my = self.y.unorm2snorm();
		let mz = self.z.unorm2snorm();
		let mw = self.w.unorm2snorm();
		Self { x: mx, y: my, z: mz, w: mw }
	}
	
	fn clamp(self, minval: Self, maxval: Self) -> Self
	{
		let mx = self.x.clamp(minval.x, maxval.x);
		let my = self.y.clamp(minval.y, maxval.y);
		let mz = self.z.clamp(minval.z, maxval.z);
		let mw = self.w.clamp(minval.w, maxval.w);
		Self { x: mx, y: my, z: mz, w:mw }
	}
}

impl<T> Lerp<T> for Quat<T> where T: Copy + One + Mul<Output=T> + Add<Output=T> + Sub<Output=T>
{
	#[inline(always)]
	fn lerp(self, b: Self, t: T) -> Self 
	{
		return self*(T::one() - t) + b*t;
	}
}

impl<T> Slerp<T> for Quat<T> where T: Vec + Math
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

impl<T> Zero for Quat<T> where T:Zero
{
	#[inline(always)]
	fn zero() -> Self
	{
		Self
		{ 
			x: T::zero(), y: T::zero(), z: T::zero(), w: T::zero() 
		}
	}
}

impl<T> One for Quat<T> where T:One
{
	#[inline(always)]
	fn one() -> Self
	{
		Self
		{ 
			x: T::one(), y: T::one(), z: T::one(), w: T::one() 
		}
	}
}

impl<T> fmt::Display for Quat<T> where T:Debug
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		write!(f, "({:?}, {:?}, {:?}, {:?})", self.x, self.y, self.z, self.w)
	}
}

impl<T> fmt::Binary for Quat<T> where T:Vec + Math
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		let len = self.length();
		let decimals = f.precision().unwrap_or(3);
		let string = format!("{:.*?}", decimals, len);
		f.pad_integral(true, "", &string)
	}
}

impl<T> From<Vec4<T>> for Quat<T> where T:Copy + Div<Output=T>
{
	fn from(v:Vec4<T>) -> Self
	{
		Self
		{
			x:v.x,
			y:v.y,
			z:v.z,
			w:v.w
		}
	}
}

impl<T> From<[T;4]> for Quat<T> where T:Copy + Div<Output=T>
{
	fn from(v:[T;4]) -> Self
	{
		Self
		{
			x:v[0],
			y:v[1],
			z:v[2],
			w:v[3],
		}
	}
}

impl<T> From<(T,T,T,T)> for Quat<T> where T:Copy + Div<Output=T>
{
	fn from(v:(T,T,T,T)) -> Self
	{
		Self
		{
			x:v.0,
			y:v.1,
			z:v.2,
			w:v.3
		}
	}
}

impl<T> AsRef<Quat<T>> for Quat<T>
{
	fn as_ref(&self) -> &Quat<T>
	{
		self
	}
}

impl<T> AsMut<Quat<T>> for Quat<T>
{
	fn as_mut(&mut self) -> &mut Quat<T>
	{
		self
	}
}