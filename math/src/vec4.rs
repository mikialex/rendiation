use std::fmt;
use std::fmt::Debug;
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use super::vec::{Vec, Math, Lerp, Slerp};
use super::vec2::Vec2;
use super::vec3::Vec3;
use super::consts::{Zero, One, UnitX, UnitY, UnitZ, UnitW};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec4<T>
{
	pub x:T,
	pub y:T,
	pub z:T,
	pub w:T,
}

impl<T> Neg for Vec4<T> where T:Neg<Output=T>
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

impl<T> Add for Vec4<T>  where T:Add<Output=T>
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

impl<T> Sub for Vec4<T> where T:Sub<Output=T>
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

impl<T> Mul<T> for Vec4<T> where T:Mul<Output=T> + Copy
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

impl<T> Mul for Vec4<T> where T:Mul<Output=T>
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

impl<T> Div<T> for Vec4<T> where T:Div<Output=T> + Copy
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

impl<T> Div for Vec4<T> where T:Div<Output=T>
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

impl<T> AddAssign for Vec4<T> where T: AddAssign<T>
{
	fn add_assign(&mut self, other: Self)
	{
		self.x += other.x;
		self.y += other.y; 
		self.z += other.z; 
		self.w += other.w; 
	}
}

impl<T> SubAssign for Vec4<T> where T: SubAssign<T>
{
	fn sub_assign(&mut self, other: Self)
	{
		self.x -= other.x;
		self.y -= other.y; 
		self.z -= other.z;
		self.w -= other.w;
	}
}

impl<T> MulAssign<T> for Vec4<T> where T: MulAssign<T> + Copy
{
	fn mul_assign(&mut self, s: T)
	{
		self.x *= s;
		self.y *= s;
		self.z *= s;
		self.w *= s;
	}
}

impl<T> MulAssign for Vec4<T> where T: MulAssign<T>
{
	fn mul_assign(&mut self, other: Self)
	{
		self.x *= other.x;
		self.y *= other.y; 
		self.z *= other.z;
		self.w *= other.w;
	}
}

impl<'a, T> MulAssign<&'a T> for Vec4<T> where T:MulAssign<T> + Copy
{
	fn mul_assign(&mut self, other: &'a T)
	{
		self.x *= *other;
		self.y *= *other;
		self.z *= *other;
		self.w *= *other;
	}
}

impl<T> DivAssign<T> for Vec4<T> where T: DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: T)
	{
		self.x /= s;
		self.y /= s;
		self.z /= s;
		self.w /= s;
	}
}

impl<T> DivAssign for Vec4<T> where T: DivAssign<T>
{
	fn div_assign(&mut self, other: Self)
	{
		self.x /= other.x;
		self.y /= other.y; 
		self.z /= other.z;
		self.w /= other.w;
	}
}

impl<'a, T> DivAssign<&'a T> for Vec4<T> where T:DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: &'a T)
	{
		self.x /= *s;
		self.y /= *s;
		self.z /= *s;
		self.w /= *s;
	}
}

impl<T> Vec4<T> where T: Copy
{
	/// Creates a new Vec4 from multiple components
	#[inline(always)]
	pub fn new(x: T, y: T, z: T, w: T) -> Self { Self { x, y, z, w } }

	#[inline(always)]
	pub fn len() -> usize 
	{
		return 4; 
	}

	#[inline(always)]
	pub fn to_tuple(&self) -> (T, T, T, T)
	{
		(self.x, self.y, self.z, self.w)
	}

	#[inline(always)]
	pub fn xx(&self) -> Vec2<T> { Vec2::new(self.x, self.x) }
	#[inline(always)]
	pub fn xy(&self) -> Vec2<T> { Vec2::new(self.x, self.y) }
	#[inline(always)]
	pub fn xz(&self) -> Vec2<T> { Vec2::new(self.x, self.z) }
	#[inline(always)]
	pub fn xw(&self) -> Vec2<T> { Vec2::new(self.x, self.w) }
	#[inline(always)]
	pub fn yx(&self) -> Vec2<T> { Vec2::new(self.y, self.x) }
	#[inline(always)]
	pub fn yy(&self) -> Vec2<T> { Vec2::new(self.y, self.y) }
	#[inline(always)]
	pub fn yz(&self) -> Vec2<T> { Vec2::new(self.y, self.z) }
	#[inline(always)]
	pub fn yw(&self) -> Vec2<T> { Vec2::new(self.y, self.w) }
	#[inline(always)]
	pub fn zx(&self) -> Vec2<T> { Vec2::new(self.z, self.x) }
	#[inline(always)]
	pub fn zy(&self) -> Vec2<T> { Vec2::new(self.z, self.y) }
	#[inline(always)]
	pub fn zz(&self) -> Vec2<T> { Vec2::new(self.z, self.z) }
	#[inline(always)]
	pub fn zw(&self) -> Vec2<T> { Vec2::new(self.z, self.w) }
	#[inline(always)]
	pub fn wx(&self) -> Vec2<T> { Vec2::new(self.w, self.x) }
	#[inline(always)]
	pub fn wy(&self) -> Vec2<T> { Vec2::new(self.w, self.y) }
	#[inline(always)]
	pub fn wz(&self) -> Vec2<T> { Vec2::new(self.w, self.z) }
	#[inline(always)]
	pub fn ww(&self) -> Vec2<T> { Vec2::new(self.w, self.w) }
	#[inline(always)]
	pub fn xxx(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.x) }
	#[inline(always)]
	pub fn xxy(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.y) }
	#[inline(always)]
	pub fn xxz(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.z) }
	#[inline(always)]
	pub fn xxw(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.w) }
	#[inline(always)]
	pub fn xyx(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.x) }
	#[inline(always)]
	pub fn xyy(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.y) }
	#[inline(always)]
	pub fn xyz(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.z) }
	#[inline(always)]
	pub fn xyw(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.w) }
	#[inline(always)]
	pub fn xzx(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.x) }
	#[inline(always)]
	pub fn xzy(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.y) }
	#[inline(always)]
	pub fn xzz(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.z) }
	#[inline(always)]
	pub fn xzw(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.w) }
	#[inline(always)]
	pub fn xwx(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.x) }
	#[inline(always)]
	pub fn xwy(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.y) }
	#[inline(always)]
	pub fn xwz(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.z) }
	#[inline(always)]
	pub fn xww(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.w) }
	#[inline(always)]
	pub fn yxx(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.x) }
	#[inline(always)]
	pub fn yxy(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.y) }
	#[inline(always)]
	pub fn yxz(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.z) }
	#[inline(always)]
	pub fn yxw(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.w) }
	#[inline(always)]
	pub fn yyx(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.x) }
	#[inline(always)]
	pub fn yyy(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.y) }
	#[inline(always)]
	pub fn yyz(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.z) }
	#[inline(always)]
	pub fn yyw(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.w) }
	#[inline(always)]
	pub fn yzx(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.x) }
	#[inline(always)]
	pub fn yzy(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.y) }
	#[inline(always)]
	pub fn yzz(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.z) }
	#[inline(always)]
	pub fn yzw(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.w) }
	#[inline(always)]
	pub fn ywx(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.x) }
	#[inline(always)]
	pub fn ywy(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.y) }
	#[inline(always)]
	pub fn ywz(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.z) }
	#[inline(always)]
	pub fn yww(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.w) }
	#[inline(always)]
	pub fn zxx(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.x) }
	#[inline(always)]
	pub fn zxy(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.y) }
	#[inline(always)]
	pub fn zxz(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.z) }
	#[inline(always)]
	pub fn zxw(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.w) }
	#[inline(always)]
	pub fn zyx(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.x) }
	#[inline(always)]
	pub fn zyy(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.y) }
	#[inline(always)]
	pub fn zyz(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.z) }
	#[inline(always)]
	pub fn zyw(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.w) }
	#[inline(always)]
	pub fn zzx(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.x) }
	#[inline(always)]
	pub fn zzy(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.y) }
	#[inline(always)]
	pub fn zzz(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.z) }
	#[inline(always)]
	pub fn zzw(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.w) }
	#[inline(always)]
	pub fn zwx(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.x) }
	#[inline(always)]
	pub fn zwy(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.y) }
	#[inline(always)]
	pub fn zwz(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.z) }
	#[inline(always)]
	pub fn zww(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.w) }
	#[inline(always)]
	pub fn wxx(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.x) }
	#[inline(always)]
	pub fn wxy(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.y) }
	#[inline(always)]
	pub fn wxz(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.z) }
	#[inline(always)]
	pub fn wxw(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.w) }
	#[inline(always)]
	pub fn wyx(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.x) }
	#[inline(always)]
	pub fn wyy(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.y) }
	#[inline(always)]
	pub fn wyz(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.z) }
	#[inline(always)]
	pub fn wyw(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.w) }
	#[inline(always)]
	pub fn wzx(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.x) }
	#[inline(always)]
	pub fn wzy(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.y) }
	#[inline(always)]
	pub fn wzz(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.z) }
	#[inline(always)]
	pub fn wzw(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.w) }
	#[inline(always)]
	pub fn wwx(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.x) }
	#[inline(always)]
	pub fn wwy(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.y) }
	#[inline(always)]
	pub fn wwz(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.z) }
	#[inline(always)]
	pub fn www(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.w) }
	#[inline(always)]
	pub fn xxxx(&self) -> Self { Self::new(self.x, self.x, self.x, self.x) }
	#[inline(always)]
	pub fn xxxy(&self) -> Self { Self::new(self.x, self.x, self.x, self.y) }
	#[inline(always)]
	pub fn xxxz(&self) -> Self { Self::new(self.x, self.x, self.x, self.z) }
	#[inline(always)]
	pub fn xxxw(&self) -> Self { Self::new(self.x, self.x, self.x, self.w) }
	#[inline(always)]
	pub fn xxyx(&self) -> Self { Self::new(self.x, self.x, self.y, self.x) }
	#[inline(always)]
	pub fn xxyy(&self) -> Self { Self::new(self.x, self.x, self.y, self.y) }
	#[inline(always)]
	pub fn xxyz(&self) -> Self { Self::new(self.x, self.x, self.y, self.z) }
	#[inline(always)]
	pub fn xxyw(&self) -> Self { Self::new(self.x, self.x, self.y, self.w) }
	#[inline(always)]
	pub fn xxzx(&self) -> Self { Self::new(self.x, self.x, self.z, self.x) }
	#[inline(always)]
	pub fn xxzy(&self) -> Self { Self::new(self.x, self.x, self.z, self.y) }
	#[inline(always)]
	pub fn xxzz(&self) -> Self { Self::new(self.x, self.x, self.z, self.z) }
	#[inline(always)]
	pub fn xxzw(&self) -> Self { Self::new(self.x, self.x, self.z, self.w) }
	#[inline(always)]
	pub fn xxwx(&self) -> Self { Self::new(self.x, self.x, self.w, self.x) }
	#[inline(always)]
	pub fn xxwy(&self) -> Self { Self::new(self.x, self.x, self.w, self.y) }
	#[inline(always)]
	pub fn xxwz(&self) -> Self { Self::new(self.x, self.x, self.w, self.z) }
	#[inline(always)]
	pub fn xxww(&self) -> Self { Self::new(self.x, self.x, self.w, self.w) }
	#[inline(always)]
	pub fn xyxx(&self) -> Self { Self::new(self.x, self.y, self.x, self.x) }
	#[inline(always)]
	pub fn xyxy(&self) -> Self { Self::new(self.x, self.y, self.x, self.y) }
	#[inline(always)]
	pub fn xyxz(&self) -> Self { Self::new(self.x, self.y, self.x, self.z) }
	#[inline(always)]
	pub fn xyxw(&self) -> Self { Self::new(self.x, self.y, self.x, self.w) }
	#[inline(always)]
	pub fn xyyx(&self) -> Self { Self::new(self.x, self.y, self.y, self.x) }
	#[inline(always)]
	pub fn xyyy(&self) -> Self { Self::new(self.x, self.y, self.y, self.y) }
	#[inline(always)]
	pub fn xyyz(&self) -> Self { Self::new(self.x, self.y, self.y, self.z) }
	#[inline(always)]
	pub fn xyyw(&self) -> Self { Self::new(self.x, self.y, self.y, self.w) }
	#[inline(always)]
	pub fn xyzx(&self) -> Self { Self::new(self.x, self.y, self.z, self.x) }
	#[inline(always)]
	pub fn xyzy(&self) -> Self { Self::new(self.x, self.y, self.z, self.y) }
	#[inline(always)]
	pub fn xyzz(&self) -> Self { Self::new(self.x, self.y, self.z, self.z) }
	#[inline(always)]
	pub fn xyzw(&self) -> Self { Self::new(self.x, self.y, self.z, self.w) }
	#[inline(always)]
	pub fn xywx(&self) -> Self { Self::new(self.x, self.y, self.w, self.x) }
	#[inline(always)]
	pub fn xywy(&self) -> Self { Self::new(self.x, self.y, self.w, self.y) }
	#[inline(always)]
	pub fn xywz(&self) -> Self { Self::new(self.x, self.y, self.w, self.z) }
	#[inline(always)]
	pub fn xyww(&self) -> Self { Self::new(self.x, self.y, self.w, self.w) }
	#[inline(always)]
	pub fn xzxx(&self) -> Self { Self::new(self.x, self.z, self.x, self.x) }
	#[inline(always)]
	pub fn xzxy(&self) -> Self { Self::new(self.x, self.z, self.x, self.y) }
	#[inline(always)]
	pub fn xzxz(&self) -> Self { Self::new(self.x, self.z, self.x, self.z) }
	#[inline(always)]
	pub fn xzxw(&self) -> Self { Self::new(self.x, self.z, self.x, self.w) }
	#[inline(always)]
	pub fn xzyx(&self) -> Self { Self::new(self.x, self.z, self.y, self.x) }
	#[inline(always)]
	pub fn xzyy(&self) -> Self { Self::new(self.x, self.z, self.y, self.y) }
	#[inline(always)]
	pub fn xzyz(&self) -> Self { Self::new(self.x, self.z, self.y, self.z) }
	#[inline(always)]
	pub fn xzyw(&self) -> Self { Self::new(self.x, self.z, self.y, self.w) }
	#[inline(always)]
	pub fn xzzx(&self) -> Self { Self::new(self.x, self.z, self.z, self.x) }
	#[inline(always)]
	pub fn xzzy(&self) -> Self { Self::new(self.x, self.z, self.z, self.y) }
	#[inline(always)]
	pub fn xzzz(&self) -> Self { Self::new(self.x, self.z, self.z, self.z) }
	#[inline(always)]
	pub fn xzzw(&self) -> Self { Self::new(self.x, self.z, self.z, self.w) }
	#[inline(always)]
	pub fn xzwx(&self) -> Self { Self::new(self.x, self.z, self.w, self.x) }
	#[inline(always)]
	pub fn xzwy(&self) -> Self { Self::new(self.x, self.z, self.w, self.y) }
	#[inline(always)]
	pub fn xzwz(&self) -> Self { Self::new(self.x, self.z, self.w, self.z) }
	#[inline(always)]
	pub fn xzww(&self) -> Self { Self::new(self.x, self.z, self.w, self.w) }
	#[inline(always)]
	pub fn xwxx(&self) -> Self { Self::new(self.x, self.w, self.x, self.x) }
	#[inline(always)]
	pub fn xwxy(&self) -> Self { Self::new(self.x, self.w, self.x, self.y) }
	#[inline(always)]
	pub fn xwxz(&self) -> Self { Self::new(self.x, self.w, self.x, self.z) }
	#[inline(always)]
	pub fn xwxw(&self) -> Self { Self::new(self.x, self.w, self.x, self.w) }
	#[inline(always)]
	pub fn xwyx(&self) -> Self { Self::new(self.x, self.w, self.y, self.x) }
	#[inline(always)]
	pub fn xwyy(&self) -> Self { Self::new(self.x, self.w, self.y, self.y) }
	#[inline(always)]
	pub fn xwyz(&self) -> Self { Self::new(self.x, self.w, self.y, self.z) }
	#[inline(always)]
	pub fn xwyw(&self) -> Self { Self::new(self.x, self.w, self.y, self.w) }
	#[inline(always)]
	pub fn xwzx(&self) -> Self { Self::new(self.x, self.w, self.z, self.x) }
	#[inline(always)]
	pub fn xwzy(&self) -> Self { Self::new(self.x, self.w, self.z, self.y) }
	#[inline(always)]
	pub fn xwzz(&self) -> Self { Self::new(self.x, self.w, self.z, self.z) }
	#[inline(always)]
	pub fn xwzw(&self) -> Self { Self::new(self.x, self.w, self.z, self.w) }
	#[inline(always)]
	pub fn xwwx(&self) -> Self { Self::new(self.x, self.w, self.w, self.x) }
	#[inline(always)]
	pub fn xwwy(&self) -> Self { Self::new(self.x, self.w, self.w, self.y) }
	#[inline(always)]
	pub fn xwwz(&self) -> Self { Self::new(self.x, self.w, self.w, self.z) }
	#[inline(always)]
	pub fn xwww(&self) -> Self { Self::new(self.x, self.w, self.w, self.w) }
	#[inline(always)]
	pub fn yxxx(&self) -> Self { Self::new(self.y, self.x, self.x, self.x) }
	#[inline(always)]
	pub fn yxxy(&self) -> Self { Self::new(self.y, self.x, self.x, self.y) }
	#[inline(always)]
	pub fn yxxz(&self) -> Self { Self::new(self.y, self.x, self.x, self.z) }
	#[inline(always)]
	pub fn yxxw(&self) -> Self { Self::new(self.y, self.x, self.x, self.w) }
	#[inline(always)]
	pub fn yxyx(&self) -> Self { Self::new(self.y, self.x, self.y, self.x) }
	#[inline(always)]
	pub fn yxyy(&self) -> Self { Self::new(self.y, self.x, self.y, self.y) }
	#[inline(always)]
	pub fn yxyz(&self) -> Self { Self::new(self.y, self.x, self.y, self.z) }
	#[inline(always)]
	pub fn yxyw(&self) -> Self { Self::new(self.y, self.x, self.y, self.w) }
	#[inline(always)]
	pub fn yxzx(&self) -> Self { Self::new(self.y, self.x, self.z, self.x) }
	#[inline(always)]
	pub fn yxzy(&self) -> Self { Self::new(self.y, self.x, self.z, self.y) }
	#[inline(always)]
	pub fn yxzz(&self) -> Self { Self::new(self.y, self.x, self.z, self.z) }
	#[inline(always)]
	pub fn yxzw(&self) -> Self { Self::new(self.y, self.x, self.z, self.w) }
	#[inline(always)]
	pub fn yxwx(&self) -> Self { Self::new(self.y, self.x, self.w, self.x) }
	#[inline(always)]
	pub fn yxwy(&self) -> Self { Self::new(self.y, self.x, self.w, self.y) }
	#[inline(always)]
	pub fn yxwz(&self) -> Self { Self::new(self.y, self.x, self.w, self.z) }
	#[inline(always)]
	pub fn yxww(&self) -> Self { Self::new(self.y, self.x, self.w, self.w) }
	#[inline(always)]
	pub fn yyxx(&self) -> Self { Self::new(self.y, self.y, self.x, self.x) }
	#[inline(always)]
	pub fn yyxy(&self) -> Self { Self::new(self.y, self.y, self.x, self.y) }
	#[inline(always)]
	pub fn yyxz(&self) -> Self { Self::new(self.y, self.y, self.x, self.z) }
	#[inline(always)]
	pub fn yyxw(&self) -> Self { Self::new(self.y, self.y, self.x, self.w) }
	#[inline(always)]
	pub fn yyyx(&self) -> Self { Self::new(self.y, self.y, self.y, self.x) }
	#[inline(always)]
	pub fn yyyy(&self) -> Self { Self::new(self.y, self.y, self.y, self.y) }
	#[inline(always)]
	pub fn yyyz(&self) -> Self { Self::new(self.y, self.y, self.y, self.z) }
	#[inline(always)]
	pub fn yyyw(&self) -> Self { Self::new(self.y, self.y, self.y, self.w) }
	#[inline(always)]
	pub fn yyzx(&self) -> Self { Self::new(self.y, self.y, self.z, self.x) }
	#[inline(always)]
	pub fn yyzy(&self) -> Self { Self::new(self.y, self.y, self.z, self.y) }
	#[inline(always)]
	pub fn yyzz(&self) -> Self { Self::new(self.y, self.y, self.z, self.z) }
	#[inline(always)]
	pub fn yyzw(&self) -> Self { Self::new(self.y, self.y, self.z, self.w) }
	#[inline(always)]
	pub fn yywx(&self) -> Self { Self::new(self.y, self.y, self.w, self.x) }
	#[inline(always)]
	pub fn yywy(&self) -> Self { Self::new(self.y, self.y, self.w, self.y) }
	#[inline(always)]
	pub fn yywz(&self) -> Self { Self::new(self.y, self.y, self.w, self.z) }
	#[inline(always)]
	pub fn yyww(&self) -> Self { Self::new(self.y, self.y, self.w, self.w) }
	#[inline(always)]
	pub fn yzxx(&self) -> Self { Self::new(self.y, self.z, self.x, self.x) }
	#[inline(always)]
	pub fn yzxy(&self) -> Self { Self::new(self.y, self.z, self.x, self.y) }
	#[inline(always)]
	pub fn yzxz(&self) -> Self { Self::new(self.y, self.z, self.x, self.z) }
	#[inline(always)]
	pub fn yzxw(&self) -> Self { Self::new(self.y, self.z, self.x, self.w) }
	#[inline(always)]
	pub fn yzyx(&self) -> Self { Self::new(self.y, self.z, self.y, self.x) }
	#[inline(always)]
	pub fn yzyy(&self) -> Self { Self::new(self.y, self.z, self.y, self.y) }
	#[inline(always)]
	pub fn yzyz(&self) -> Self { Self::new(self.y, self.z, self.y, self.z) }
	#[inline(always)]
	pub fn yzyw(&self) -> Self { Self::new(self.y, self.z, self.y, self.w) }
	#[inline(always)]
	pub fn yzzx(&self) -> Self { Self::new(self.y, self.z, self.z, self.x) }
	#[inline(always)]
	pub fn yzzy(&self) -> Self { Self::new(self.y, self.z, self.z, self.y) }
	#[inline(always)]
	pub fn yzzz(&self) -> Self { Self::new(self.y, self.z, self.z, self.z) }
	#[inline(always)]
	pub fn yzzw(&self) -> Self { Self::new(self.y, self.z, self.z, self.w) }
	#[inline(always)]
	pub fn yzwx(&self) -> Self { Self::new(self.y, self.z, self.w, self.x) }
	#[inline(always)]
	pub fn yzwy(&self) -> Self { Self::new(self.y, self.z, self.w, self.y) }
	#[inline(always)]
	pub fn yzwz(&self) -> Self { Self::new(self.y, self.z, self.w, self.z) }
	#[inline(always)]
	pub fn yzww(&self) -> Self { Self::new(self.y, self.z, self.w, self.w) }
	#[inline(always)]
	pub fn ywxx(&self) -> Self { Self::new(self.y, self.w, self.x, self.x) }
	#[inline(always)]
	pub fn ywxy(&self) -> Self { Self::new(self.y, self.w, self.x, self.y) }
	#[inline(always)]
	pub fn ywxz(&self) -> Self { Self::new(self.y, self.w, self.x, self.z) }
	#[inline(always)]
	pub fn ywxw(&self) -> Self { Self::new(self.y, self.w, self.x, self.w) }
	#[inline(always)]
	pub fn ywyx(&self) -> Self { Self::new(self.y, self.w, self.y, self.x) }
	#[inline(always)]
	pub fn ywyy(&self) -> Self { Self::new(self.y, self.w, self.y, self.y) }
	#[inline(always)]
	pub fn ywyz(&self) -> Self { Self::new(self.y, self.w, self.y, self.z) }
	#[inline(always)]
	pub fn ywyw(&self) -> Self { Self::new(self.y, self.w, self.y, self.w) }
	#[inline(always)]
	pub fn ywzx(&self) -> Self { Self::new(self.y, self.w, self.z, self.x) }
	#[inline(always)]
	pub fn ywzy(&self) -> Self { Self::new(self.y, self.w, self.z, self.y) }
	#[inline(always)]
	pub fn ywzz(&self) -> Self { Self::new(self.y, self.w, self.z, self.z) }
	#[inline(always)]
	pub fn ywzw(&self) -> Self { Self::new(self.y, self.w, self.z, self.w) }
	#[inline(always)]
	pub fn ywwx(&self) -> Self { Self::new(self.y, self.w, self.w, self.x) }
	#[inline(always)]
	pub fn ywwy(&self) -> Self { Self::new(self.y, self.w, self.w, self.y) }
	#[inline(always)]
	pub fn ywwz(&self) -> Self { Self::new(self.y, self.w, self.w, self.z) }
	#[inline(always)]
	pub fn ywww(&self) -> Self { Self::new(self.y, self.w, self.w, self.w) }
	#[inline(always)]
	pub fn zxxx(&self) -> Self { Self::new(self.z, self.x, self.x, self.x) }
	#[inline(always)]
	pub fn zxxy(&self) -> Self { Self::new(self.z, self.x, self.x, self.y) }
	#[inline(always)]
	pub fn zxxz(&self) -> Self { Self::new(self.z, self.x, self.x, self.z) }
	#[inline(always)]
	pub fn zxxw(&self) -> Self { Self::new(self.z, self.x, self.x, self.w) }
	#[inline(always)]
	pub fn zxyx(&self) -> Self { Self::new(self.z, self.x, self.y, self.x) }
	#[inline(always)]
	pub fn zxyy(&self) -> Self { Self::new(self.z, self.x, self.y, self.y) }
	#[inline(always)]
	pub fn zxyz(&self) -> Self { Self::new(self.z, self.x, self.y, self.z) }
	#[inline(always)]
	pub fn zxyw(&self) -> Self { Self::new(self.z, self.x, self.y, self.w) }
	#[inline(always)]
	pub fn zxzx(&self) -> Self { Self::new(self.z, self.x, self.z, self.x) }
	#[inline(always)]
	pub fn zxzy(&self) -> Self { Self::new(self.z, self.x, self.z, self.y) }
	#[inline(always)]
	pub fn zxzz(&self) -> Self { Self::new(self.z, self.x, self.z, self.z) }
	#[inline(always)]
	pub fn zxzw(&self) -> Self { Self::new(self.z, self.x, self.z, self.w) }
	#[inline(always)]
	pub fn zxwx(&self) -> Self { Self::new(self.z, self.x, self.w, self.x) }
	#[inline(always)]
	pub fn zxwy(&self) -> Self { Self::new(self.z, self.x, self.w, self.y) }
	#[inline(always)]
	pub fn zxwz(&self) -> Self { Self::new(self.z, self.x, self.w, self.z) }
	#[inline(always)]
	pub fn zxww(&self) -> Self { Self::new(self.z, self.x, self.w, self.w) }
	#[inline(always)]
	pub fn zyxx(&self) -> Self { Self::new(self.z, self.y, self.x, self.x) }
	#[inline(always)]
	pub fn zyxy(&self) -> Self { Self::new(self.z, self.y, self.x, self.y) }
	#[inline(always)]
	pub fn zyxz(&self) -> Self { Self::new(self.z, self.y, self.x, self.z) }
	#[inline(always)]
	pub fn zyxw(&self) -> Self { Self::new(self.z, self.y, self.x, self.w) }
	#[inline(always)]
	pub fn zyyx(&self) -> Self { Self::new(self.z, self.y, self.y, self.x) }
	#[inline(always)]
	pub fn zyyy(&self) -> Self { Self::new(self.z, self.y, self.y, self.y) }
	#[inline(always)]
	pub fn zyyz(&self) -> Self { Self::new(self.z, self.y, self.y, self.z) }
	#[inline(always)]
	pub fn zyyw(&self) -> Self { Self::new(self.z, self.y, self.y, self.w) }
	#[inline(always)]
	pub fn zyzx(&self) -> Self { Self::new(self.z, self.y, self.z, self.x) }
	#[inline(always)]
	pub fn zyzy(&self) -> Self { Self::new(self.z, self.y, self.z, self.y) }
	#[inline(always)]
	pub fn zyzz(&self) -> Self { Self::new(self.z, self.y, self.z, self.z) }
	#[inline(always)]
	pub fn zyzw(&self) -> Self { Self::new(self.z, self.y, self.z, self.w) }
	#[inline(always)]
	pub fn zywx(&self) -> Self { Self::new(self.z, self.y, self.w, self.x) }
	#[inline(always)]
	pub fn zywy(&self) -> Self { Self::new(self.z, self.y, self.w, self.y) }
	#[inline(always)]
	pub fn zywz(&self) -> Self { Self::new(self.z, self.y, self.w, self.z) }
	#[inline(always)]
	pub fn zyww(&self) -> Self { Self::new(self.z, self.y, self.w, self.w) }
	#[inline(always)]
	pub fn zzxx(&self) -> Self { Self::new(self.z, self.z, self.x, self.x) }
	#[inline(always)]
	pub fn zzxy(&self) -> Self { Self::new(self.z, self.z, self.x, self.y) }
	#[inline(always)]
	pub fn zzxz(&self) -> Self { Self::new(self.z, self.z, self.x, self.z) }
	#[inline(always)]
	pub fn zzxw(&self) -> Self { Self::new(self.z, self.z, self.x, self.w) }
	#[inline(always)]
	pub fn zzyx(&self) -> Self { Self::new(self.z, self.z, self.y, self.x) }
	#[inline(always)]
	pub fn zzyy(&self) -> Self { Self::new(self.z, self.z, self.y, self.y) }
	#[inline(always)]
	pub fn zzyz(&self) -> Self { Self::new(self.z, self.z, self.y, self.z) }
	#[inline(always)]
	pub fn zzyw(&self) -> Self { Self::new(self.z, self.z, self.y, self.w) }
	#[inline(always)]
	pub fn zzzx(&self) -> Self { Self::new(self.z, self.z, self.z, self.x) }
	#[inline(always)]
	pub fn zzzy(&self) -> Self { Self::new(self.z, self.z, self.z, self.y) }
	#[inline(always)]
	pub fn zzzz(&self) -> Self { Self::new(self.z, self.z, self.z, self.z) }
	#[inline(always)]
	pub fn zzzw(&self) -> Self { Self::new(self.z, self.z, self.z, self.w) }
	#[inline(always)]
	pub fn zzwx(&self) -> Self { Self::new(self.z, self.z, self.w, self.x) }
	#[inline(always)]
	pub fn zzwy(&self) -> Self { Self::new(self.z, self.z, self.w, self.y) }
	#[inline(always)]
	pub fn zzwz(&self) -> Self { Self::new(self.z, self.z, self.w, self.z) }
	#[inline(always)]
	pub fn zzww(&self) -> Self { Self::new(self.z, self.z, self.w, self.w) }
	#[inline(always)]
	pub fn zwxx(&self) -> Self { Self::new(self.z, self.w, self.x, self.x) }
	#[inline(always)]
	pub fn zwxy(&self) -> Self { Self::new(self.z, self.w, self.x, self.y) }
	#[inline(always)]
	pub fn zwxz(&self) -> Self { Self::new(self.z, self.w, self.x, self.z) }
	#[inline(always)]
	pub fn zwxw(&self) -> Self { Self::new(self.z, self.w, self.x, self.w) }
	#[inline(always)]
	pub fn zwyx(&self) -> Self { Self::new(self.z, self.w, self.y, self.x) }
	#[inline(always)]
	pub fn zwyy(&self) -> Self { Self::new(self.z, self.w, self.y, self.y) }
	#[inline(always)]
	pub fn zwyz(&self) -> Self { Self::new(self.z, self.w, self.y, self.z) }
	#[inline(always)]
	pub fn zwyw(&self) -> Self { Self::new(self.z, self.w, self.y, self.w) }
	#[inline(always)]
	pub fn zwzx(&self) -> Self { Self::new(self.z, self.w, self.z, self.x) }
	#[inline(always)]
	pub fn zwzy(&self) -> Self { Self::new(self.z, self.w, self.z, self.y) }
	#[inline(always)]
	pub fn zwzz(&self) -> Self { Self::new(self.z, self.w, self.z, self.z) }
	#[inline(always)]
	pub fn zwzw(&self) -> Self { Self::new(self.z, self.w, self.z, self.w) }
	#[inline(always)]
	pub fn zwwx(&self) -> Self { Self::new(self.z, self.w, self.w, self.x) }
	#[inline(always)]
	pub fn zwwy(&self) -> Self { Self::new(self.z, self.w, self.w, self.y) }
	#[inline(always)]
	pub fn zwwz(&self) -> Self { Self::new(self.z, self.w, self.w, self.z) }
	#[inline(always)]
	pub fn zwww(&self) -> Self { Self::new(self.z, self.w, self.w, self.w) }
	#[inline(always)]
	pub fn wxxx(&self) -> Self { Self::new(self.w, self.x, self.x, self.x) }
	#[inline(always)]
	pub fn wxxy(&self) -> Self { Self::new(self.w, self.x, self.x, self.y) }
	#[inline(always)]
	pub fn wxxz(&self) -> Self { Self::new(self.w, self.x, self.x, self.z) }
	#[inline(always)]
	pub fn wxxw(&self) -> Self { Self::new(self.w, self.x, self.x, self.w) }
	#[inline(always)]
	pub fn wxyx(&self) -> Self { Self::new(self.w, self.x, self.y, self.x) }
	#[inline(always)]
	pub fn wxyy(&self) -> Self { Self::new(self.w, self.x, self.y, self.y) }
	#[inline(always)]
	pub fn wxyz(&self) -> Self { Self::new(self.w, self.x, self.y, self.z) }
	#[inline(always)]
	pub fn wxyw(&self) -> Self { Self::new(self.w, self.x, self.y, self.w) }
	#[inline(always)]
	pub fn wxzx(&self) -> Self { Self::new(self.w, self.x, self.z, self.x) }
	#[inline(always)]
	pub fn wxzy(&self) -> Self { Self::new(self.w, self.x, self.z, self.y) }
	#[inline(always)]
	pub fn wxzz(&self) -> Self { Self::new(self.w, self.x, self.z, self.z) }
	#[inline(always)]
	pub fn wxzw(&self) -> Self { Self::new(self.w, self.x, self.z, self.w) }
	#[inline(always)]
	pub fn wxwx(&self) -> Self { Self::new(self.w, self.x, self.w, self.x) }
	#[inline(always)]
	pub fn wxwy(&self) -> Self { Self::new(self.w, self.x, self.w, self.y) }
	#[inline(always)]
	pub fn wxwz(&self) -> Self { Self::new(self.w, self.x, self.w, self.z) }
	#[inline(always)]
	pub fn wxww(&self) -> Self { Self::new(self.w, self.x, self.w, self.w) }
	#[inline(always)]
	pub fn wyxx(&self) -> Self { Self::new(self.w, self.y, self.x, self.x) }
	#[inline(always)]
	pub fn wyxy(&self) -> Self { Self::new(self.w, self.y, self.x, self.y) }
	#[inline(always)]
	pub fn wyxz(&self) -> Self { Self::new(self.w, self.y, self.x, self.z) }
	#[inline(always)]
	pub fn wyxw(&self) -> Self { Self::new(self.w, self.y, self.x, self.w) }
	#[inline(always)]
	pub fn wyyx(&self) -> Self { Self::new(self.w, self.y, self.y, self.x) }
	#[inline(always)]
	pub fn wyyy(&self) -> Self { Self::new(self.w, self.y, self.y, self.y) }
	#[inline(always)]
	pub fn wyyz(&self) -> Self { Self::new(self.w, self.y, self.y, self.z) }
	#[inline(always)]
	pub fn wyyw(&self) -> Self { Self::new(self.w, self.y, self.y, self.w) }
	#[inline(always)]
	pub fn wyzx(&self) -> Self { Self::new(self.w, self.y, self.z, self.x) }
	#[inline(always)]
	pub fn wyzy(&self) -> Self { Self::new(self.w, self.y, self.z, self.y) }
	#[inline(always)]
	pub fn wyzz(&self) -> Self { Self::new(self.w, self.y, self.z, self.z) }
	#[inline(always)]
	pub fn wyzw(&self) -> Self { Self::new(self.w, self.y, self.z, self.w) }
	#[inline(always)]
	pub fn wywx(&self) -> Self { Self::new(self.w, self.y, self.w, self.x) }
	#[inline(always)]
	pub fn wywy(&self) -> Self { Self::new(self.w, self.y, self.w, self.y) }
	#[inline(always)]
	pub fn wywz(&self) -> Self { Self::new(self.w, self.y, self.w, self.z) }
	#[inline(always)]
	pub fn wyww(&self) -> Self { Self::new(self.w, self.y, self.w, self.w) }
	#[inline(always)]
	pub fn wzxx(&self) -> Self { Self::new(self.w, self.z, self.x, self.x) }
	#[inline(always)]
	pub fn wzxy(&self) -> Self { Self::new(self.w, self.z, self.x, self.y) }
	#[inline(always)]
	pub fn wzxz(&self) -> Self { Self::new(self.w, self.z, self.x, self.z) }
	#[inline(always)]
	pub fn wzxw(&self) -> Self { Self::new(self.w, self.z, self.x, self.w) }
	#[inline(always)]
	pub fn wzyx(&self) -> Self { Self::new(self.w, self.z, self.y, self.x) }
	#[inline(always)]
	pub fn wzyy(&self) -> Self { Self::new(self.w, self.z, self.y, self.y) }
	#[inline(always)]
	pub fn wzyz(&self) -> Self { Self::new(self.w, self.z, self.y, self.z) }
	#[inline(always)]
	pub fn wzyw(&self) -> Self { Self::new(self.w, self.z, self.y, self.w) }
	#[inline(always)]
	pub fn wzzx(&self) -> Self { Self::new(self.w, self.z, self.z, self.x) }
	#[inline(always)]
	pub fn wzzy(&self) -> Self { Self::new(self.w, self.z, self.z, self.y) }
	#[inline(always)]
	pub fn wzzz(&self) -> Self { Self::new(self.w, self.z, self.z, self.z) }
	#[inline(always)]
	pub fn wzzw(&self) -> Self { Self::new(self.w, self.z, self.z, self.w) }
	#[inline(always)]
	pub fn wzwx(&self) -> Self { Self::new(self.w, self.z, self.w, self.x) }
	#[inline(always)]
	pub fn wzwy(&self) -> Self { Self::new(self.w, self.z, self.w, self.y) }
	#[inline(always)]
	pub fn wzwz(&self) -> Self { Self::new(self.w, self.z, self.w, self.z) }
	#[inline(always)]
	pub fn wzww(&self) -> Self { Self::new(self.w, self.z, self.w, self.w) }
	#[inline(always)]
	pub fn wwxx(&self) -> Self { Self::new(self.w, self.w, self.x, self.x) }
	#[inline(always)]
	pub fn wwxy(&self) -> Self { Self::new(self.w, self.w, self.x, self.y) }
	#[inline(always)]
	pub fn wwxz(&self) -> Self { Self::new(self.w, self.w, self.x, self.z) }
	#[inline(always)]
	pub fn wwxw(&self) -> Self { Self::new(self.w, self.w, self.x, self.w) }
	#[inline(always)]
	pub fn wwyx(&self) -> Self { Self::new(self.w, self.w, self.y, self.x) }
	#[inline(always)]
	pub fn wwyy(&self) -> Self { Self::new(self.w, self.w, self.y, self.y) }
	#[inline(always)]
	pub fn wwyz(&self) -> Self { Self::new(self.w, self.w, self.y, self.z) }
	#[inline(always)]
	pub fn wwyw(&self) -> Self { Self::new(self.w, self.w, self.y, self.w) }
	#[inline(always)]
	pub fn wwzx(&self) -> Self { Self::new(self.w, self.w, self.z, self.x) }
	#[inline(always)]
	pub fn wwzy(&self) -> Self { Self::new(self.w, self.w, self.z, self.y) }
	#[inline(always)]
	pub fn wwzz(&self) -> Self { Self::new(self.w, self.w, self.z, self.z) }
	#[inline(always)]
	pub fn wwzw(&self) -> Self { Self::new(self.w, self.w, self.z, self.w) }
	#[inline(always)]
	pub fn wwwx(&self) -> Self { Self::new(self.w, self.w, self.w, self.x) }
	#[inline(always)]
	pub fn wwwy(&self) -> Self { Self::new(self.w, self.w, self.w, self.y) }
	#[inline(always)]
	pub fn wwwz(&self) -> Self { Self::new(self.w, self.w, self.w, self.z) }
	#[inline(always)]
	pub fn wwww(&self) -> Self { Self::new(self.w, self.w, self.w, self.w) }
}

impl<T> Vec4<T> where T:Vec + Math
{
	#[inline]
	pub fn dot(&self, b: Self) -> T 
	{
		return self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w;
	}
	
	#[inline]
	pub fn cross(&self, b: Self) -> Self 
	{
		Vec4
		{
			x:self.w * b.x + self.x * b.w + self.z * b.y - self.y * b.z,
			y:self.w * b.y + self.y * b.w + self.x * b.z - self.z * b.x,
			z:self.w * b.z + self.z * b.w + self.y * b.x - self.x * b.y,
			w:self.w * b.w - self.x * b.x - self.y * b.y - self.z * b.z,
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

impl<T> Math for Vec4<T> where T:Copy + Math
{
	#[inline]
	fn abs(self) -> Self
	{
		let mx = self.x.abs();
		let my = self.y.abs();
		let mz = self.z.abs();
		let mw = self.w.abs();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn recip(self) -> Self
	{
		let mx = self.x.recip();
		let my = self.y.recip();
		let mz = self.z.recip();
		let mw = self.w.recip();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn sqrt(self) -> Self
	{
		let mx = self.x.sqrt();
		let my = self.y.sqrt();
		let mz = self.z.sqrt();
		let mw = self.w.sqrt();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn rsqrt(self) -> Self
	{
		let mx = self.x.rsqrt();
		let my = self.y.rsqrt();
		let mz = self.z.rsqrt();
		let mw = self.w.rsqrt();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn sin(self) -> Self
	{
		let mx = self.x.sin();
		let my = self.y.sin();
		let mz = self.z.sin();
		let mw = self.w.sin();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn cos(self) -> Self
	{
		let mx = self.x.cos();
		let my = self.y.cos();
		let mz = self.z.cos();
		let mw = self.w.cos();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn tan(self) -> Self
	{
		let mx = self.x.tan();
		let my = self.y.tan();
		let mz = self.z.tan();
		let mw = self.w.tan();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn sincos(self) -> (Vec4<T>, Vec4<T>)
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

	#[inline]
	fn acos(self) -> Self
	{
		let mx = self.x.acos();
		let my = self.y.acos();
		let mz = self.z.acos();
		let mw = self.w.acos();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn asin(self) -> Self
	{
		let mx = self.x.asin();
		let my = self.y.asin();
		let mz = self.z.asin();
		let mw = self.w.asin();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn atan(self) -> Self
	{
		let mx = self.x.atan();
		let my = self.y.atan();
		let mz = self.z.atan();
		let mw = self.w.atan();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn exp(self) -> Self
	{
		let mx = self.x.exp();
		let my = self.y.exp();
		let mz = self.z.exp();
		let mw = self.w.exp();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn exp2(self) -> Self
	{
		let mx = self.x.exp2();
		let my = self.y.exp2();
		let mz = self.z.exp2();
		let mw = self.w.exp2();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn log(self, _rhs: Self) -> Self
	{
		let mx = self.x.log(_rhs.x);
		let my = self.y.log(_rhs.y);
		let mz = self.z.log(_rhs.z);
		let mw = self.w.log(_rhs.w);
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn log2(self) -> Self
	{
		let mx = self.x.log2();
		let my = self.y.log2();
		let mz = self.z.log2();
		let mw = self.w.log2();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn log10(self) -> Self
	{
		let mx = self.x.log10();
		let my = self.y.log10();
		let mz = self.z.log10();
		let mw = self.w.log10();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn to_radians(self) -> Self
	{
		let mx = self.x.to_radians();
		let my = self.y.to_radians();
		let mz = self.z.to_radians();
		let mw = self.w.to_radians();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn to_degrees(self) -> Self
	{
		let mx = self.x.to_degrees();
		let my = self.y.to_degrees();
		let mz = self.z.to_degrees();
		let mw = self.w.to_degrees();
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn min(self, _rhs: Self) -> Self
	{
		let mx = self.x.min(_rhs.x);
		let my = self.y.min(_rhs.y);
		let mz = self.z.min(_rhs.z);
		let mw = self.w.min(_rhs.x);
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
	fn max(self, _rhs: Self) -> Self
	{
		let mx = self.x.max(_rhs.x);
		let my = self.y.max(_rhs.y);
		let mz = self.z.max(_rhs.z);
		let mw = self.w.max(_rhs.w);
		Self { x: mx, y: my, z: mz, w:mw }
	}

	#[inline]
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

	#[inline]
	fn clamp(self, minval: Self, maxval: Self) -> Self
	{
		let mx = self.x.clamp(minval.x, maxval.x);
		let my = self.y.clamp(minval.y, maxval.y);
		let mz = self.z.clamp(minval.z, maxval.z);
		let mw = self.w.clamp(minval.w, maxval.w);
		Self { x: mx, y: my, z: mz, w:mw }
	}
}

impl<T> Lerp<T> for Vec4<T> where T: Copy + One + Mul<Output=T> + Add<Output=T> + Sub<Output=T>
{
	#[inline(always)]
	fn lerp(self, b: Self, t: T) -> Self 
	{
		return self*(T::one() - t) + b*t;
	}
}

impl<T> Slerp<T> for Vec4<T> where T: Vec + Math
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

impl<T> Zero for Vec4<T> where T:Zero
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

impl<T> One for Vec4<T> where T:One
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

impl<T> UnitX for Vec4<T> where T:One + Zero
{
	#[inline(always)]
	fn unit_x() -> Self
	{
		Self
		{ 
			x: T::one(), y: T::zero(), z: T::zero(), w: T::one()
		}
	}
}

impl<T> UnitY for Vec4<T> where T:One + Zero
{
	#[inline(always)]
	fn unit_y() -> Self
	{
		Self
		{ 
			x: T::zero(), y: T::one(), z: T::zero(), w: T::one()
		}
	}
}

impl<T> UnitZ for Vec4<T> where T:One + Zero
{
	#[inline(always)]
	fn unit_z() -> Self
	{
		Self
		{ 
			x: T::zero(), y: T::one(), z: T::zero(), w: T::one()
		}
	}
}

impl<T> UnitW for Vec4<T> where T:One + Zero
{
	#[inline(always)]
	fn unit_w() -> Self
	{
		Self
		{ 
			x: T::zero(), y: T::zero(), z: T::zero(), w: T::one()
		}
	}
}

impl<T> fmt::Display for Vec4<T> where T:Debug
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		write!(f, "({:?}, {:?}, {:?}, {:?})", self.x, self.y, self.z, self.w)
	}
}

impl<T> fmt::Binary for Vec4<T> where T:Vec + Math
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		let len = self.length();
		let decimals = f.precision().unwrap_or(3);
		let string = format!("{:.*?}", decimals, len);
		f.pad_integral(true, "", &string)
	}
}

impl<T> From<Vec3<T>> for Vec4<T> where T:Copy + One
{
	fn from(v:Vec3<T>) -> Self
	{
		Self
		{
			x:v.x,
			y:v.y,
			z:v.z,
			w:T::one()
		}
	}
}

impl<T> From<Vec4<T>> for Vec3<T> where T:Copy + Div<Output=T>
{
	fn from(v:Vec4<T>) -> Self
	{
		Self
		{
			x:v.x / v.w,
			y:v.y / v.w,
			z:v.z / v.w,
		}
	}
}

impl<T> From<[T;4]> for Vec4<T> where T:Copy
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

impl<T> From<(T,T,T,T)> for Vec4<T> where T:Copy
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

impl<T> AsRef<Vec4<T>> for Vec4<T>
{
	fn as_ref(&self) -> &Vec4<T>
	{
		self
	}
}

impl<T> AsMut<Vec4<T>> for Vec4<T>
{
	fn as_mut(&mut self) -> &mut Vec4<T>
	{
		self
	}
}