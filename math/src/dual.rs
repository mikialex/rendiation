use std::fmt;
use std::fmt::Debug;
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use super::vec::{Vec, Math, Lerp, Slerp};
use super::quat::Quat;
use super::vec3::Vec3;
use super::consts::{Zero, One};

// http://wscg.zcu.cz/wscg2012/short/a29-full.pdf
#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Dual<T>
{
	pub real:Quat<T>,
	pub dual:Quat<T>,
}

impl<T> Neg for Dual<T> where T:Neg<Output=T>
{
	type Output = Self;

	fn neg(self) -> Self
	{
		Self
		{
			real:self.real,
			dual:self.dual,
		}
	}
}

impl<T> Add for Dual<T>  where T:Add<Output=T>
{
	type Output = Self;

	fn add(self, other: Self) -> Self
	{
		Self
		{
			real:self.real + other.real,
			dual:self.dual + other.dual,
		}
	}
}

impl<T> Sub for Dual<T>  where T:Sub<Output=T>
{
	type Output = Self;

	fn sub(self, other: Self) -> Self
	{
		Self
		{
			real:self.real - other.real,
			dual:self.dual - other.dual,
		}
	}
}

impl<T> Mul for Dual<T> where T:Mul<Output=T> + Add<Output=T> + Copy
{
	type Output = Self;

	fn mul(self, other: Self) -> Self
	{
		Self
		{
			real:self.real * other.real,
			dual:(self.real * other.dual) + (self.dual * other.real),
		}
	}
}

impl<T> Mul<T> for Dual<T> where T:Mul<Output=T> + Copy
{
	type Output = Self;

	fn mul(self, s: T) -> Self
	{
		Self
		{
			real:self.real * s,
			dual:self.dual * s,
		}
	}
}

impl<T> Div<T> for Dual<T> where T:Div<Output=T> + Copy
{
	type Output = Self;

	fn div(self, s: T) -> Self
	{
		Self
		{
			real:self.real / s,
			dual:self.dual / s,
		}
	}
}

impl<T> AddAssign for Dual<T> where T: AddAssign<T>
{
	fn add_assign(&mut self, other: Self)
	{
		self.real += other.real;
		self.dual += other.dual;
	}
}

impl<T> SubAssign for Dual<T> where T: SubAssign<T>
{
	fn sub_assign(&mut self, other: Self)
	{
		self.real -= other.real;
		self.dual -= other.dual;
	}
}

impl<T> MulAssign<T> for Dual<T> where T: MulAssign<T> + Copy
{
	fn mul_assign(&mut self, s: T)
	{
		self.real *= s;
		self.dual *= s;
	}
}

impl<'a, T> MulAssign<&'a T> for Dual<T> where T:MulAssign<T> + Copy
{
	fn mul_assign(&mut self, s: &'a T)
	{
		self.real *= s;
		self.dual *= s;
	}
}

impl<T> DivAssign<T> for Dual<T> where T: DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: T)
	{
		self.real /= s;
		self.dual /= s;
	}
}

impl<'a, T> DivAssign<&'a T> for Dual<T> where T:DivAssign<T> + Copy
{
	fn div_assign(&mut self, s: &'a T)
	{
		self.real /= s;
		self.dual /= s;
	}
}

impl<T> Dual<T> where T:Copy
{
	/// Creates a new Dual from two quaternions
	pub fn new(real:Quat<T>, dual:Quat<T>) -> Self 
	{
		Self
		{
			real:real,
			dual:dual
		}
	}
}

impl<T> Dual<T> where T:Vec + Math
{
	pub fn from_transform(rotation:Quat<T>, t:Vec3<T>) -> Self
	{
		Self
		{
			real:rotation,
			dual:Quat::new(t.x,t.y,t.z,T::zero()) * rotation * T::onehalf()
		}
	}

	pub fn dot(&self, b: Self) -> T 
	{
		return self.real.dot(b.real);
	}
	
	pub fn length2(&self) -> T 
	{
		return self.dot(*self);
	}
	
	pub fn length(&self) -> T 
	{
		return self.length2().sqrt();
	}

	pub fn normalize(&self) -> Self 
	{
		let mag_sq = self.real.length2();
		if mag_sq.gt(T::zero())
		{
			let inv_sqrt = T::one() / mag_sq.sqrt();
			return *self * inv_sqrt;
		}

		return *self;
	}

	pub fn rotation(&self) -> Quat<T>
	{
		self.real
	}

	pub fn translate(&self) -> Vec3<T>
	{
		let t = self.dual * T::two() * self.real.conj();
		Vec3::new(t.x, t.y, t.z)
	}

	pub fn conj(&self) -> Self
	{
		Self
		{ 
			real: self.real.conj(),
			dual: self.dual.conj(),
		}
	}

	pub fn conjugate(&self) -> Self
	{
		Self
		{ 
			real: self.real.conjugate(),
			dual: self.dual.conjugate(),
		}
	}

	pub fn inverse(&self) -> Self
	{
		self.conjugate()
	}
}

impl<T> Lerp<T> for Dual<T> where T: Copy + One + Mul<Output=T> + Add<Output=T> + Sub<Output=T>
{
	#[inline(always)]
	fn lerp(self, b: Self, t: T) -> Self 
	{
		return self*(T::one() - t) + b*t;
	}
}

// http://dcgi.felk.cvut.cz/home/zara/papers/TCD-CS-2006-46.pdf
impl<T> Slerp<T> for Dual<T> where T: Vec + Math
{
	fn slerp(self, q2: Self, factor: T) -> Self 
	{
		let dot = self.dot(q2);

		let s = T::one() - factor;
		let t = if dot.gt(T::zero()) { factor } else { -factor };
		let q = self * s + q2 * t;

		q.normalize()
	}
}

impl<T> Zero for Dual<T> where T:Zero
{
	#[inline(always)]
	fn zero() -> Self
	{
		Self
		{ 
			real:Quat::zero(),
			dual:Quat::zero(),
		}
	}
}

impl<T> One for Dual<T> where T:One
{
	#[inline(always)]
	fn one() -> Self
	{
		Self
		{ 
			real:Quat::one(),
			dual:Quat::one(),
		}
	}
}

impl<T> fmt::Display for Dual<T> where T:Debug
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		write!(f, "({:?}, {:?})", self.real, self.dual)
	}
}

impl<T> AsRef<Dual<T>> for Dual<T>
{
	fn as_ref(&self) -> &Dual<T>
	{
		self
	}
}

impl<T> AsMut<Dual<T>> for Dual<T>
{
	fn as_mut(&mut self) -> &mut Dual<T>
	{
		self
	}
}