use std::fmt;
use std::marker::PhantomData;
use serde::ser::SerializeTuple;
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Visitor, SeqAccess, Error};

use super::vec2::Vec2;
use super::vec3::Vec3;
use super::vec4::Vec4;
use super::quat::Quat;
use super::dual::Dual;
use super::mat2::Mat2;
use super::mat3::Mat3;
use super::mat4::Mat4;

struct Vec2Visitor<T>(PhantomData<T>);
impl<T> Vec2Visitor<T>
{
	fn new() -> Self { Vec2Visitor(PhantomData) }
}

struct Vec3Visitor<T>(PhantomData<T>);
impl<T> Vec3Visitor<T>
{
	fn new() -> Self { Vec3Visitor(PhantomData) }
}

struct Vec4Visitor<T>(PhantomData<T>);
impl<T> Vec4Visitor<T>
{
	fn new() -> Self { Vec4Visitor(PhantomData) }
}

struct QuatVisitor<T>(PhantomData<T>);
impl<T> QuatVisitor<T>
{
	fn new() -> Self { QuatVisitor(PhantomData) }
}

struct DualVisitor<T>(PhantomData<T>);
impl<T> DualVisitor<T>
{
	fn new() -> Self { DualVisitor(PhantomData) }
}

struct Mat2Visitor<T>(PhantomData<T>);
impl<T> Mat2Visitor<T>
{
	fn new() -> Self { Mat2Visitor(PhantomData) }
}

struct Mat3Visitor<T>(PhantomData<T>);
impl<T> Mat3Visitor<T>
{
	fn new() -> Self { Mat3Visitor(PhantomData) }
}

struct Mat4Visitor<T>(PhantomData<T>);
impl<T> Mat4Visitor<T>
{
	fn new() -> Self { Mat4Visitor(PhantomData) }
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for Vec2Visitor<T>
{
	type Value = Vec2<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 2")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let x: Option<T> = a.next_element()?;
		let y: Option<T> = a.next_element()?;
		let z: Option<T> = a.next_element()?;
		let w: Option<T> = a.next_element()?;

		match (x, y, z, w)
		{
			(Some(_), None, None, None) =>          Err(A::Error::invalid_length(1, &"Sequence of length 2")),
			(Some(x), Some(y), None, None) =>       Ok(Vec2::new(x, y)),
			(Some(_), Some(_), Some(_), None) =>    Err(A::Error::invalid_length(2, &"Sequence of length 2")),
			(Some(_), Some(_), Some(_), Some(_)) => Err(A::Error::invalid_length(4, &"Sequence of length 2")),
			_ =>                                    Err(A::Error::custom("Expected array of length 2, found nothing")),
		}
	}
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for Vec3Visitor<T>
{
	type Value = Vec3<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 3")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let x: Option<T> = a.next_element()?;
		let y: Option<T> = a.next_element()?;
		let z: Option<T> = a.next_element()?;
		let w: Option<T> = a.next_element()?;

		match (x, y, z, w)
		{
			(Some(_), None, None, None) =>          Err(A::Error::invalid_length(1, &"Sequence of length 3")),
			(Some(_), Some(_), None, None) =>       Err(A::Error::invalid_length(2, &"Sequence of length 3")),
			(Some(x), Some(y), Some(z), None) =>    Ok(Vec3::new(x, y, z)),
			(Some(_), Some(_), Some(_), Some(_)) => Err(A::Error::invalid_length(4, &"Sequence of length 3")),
			_ =>                                    Err(A::Error::custom("Expected array of length 3, found nothing")),
		}
	}
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for Vec4Visitor<T>
{
	type Value = Vec4<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 4")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let x: Option<T> = a.next_element()?;
		let y: Option<T> = a.next_element()?;
		let z: Option<T> = a.next_element()?;
		let w: Option<T> = a.next_element()?;

		match (x, y, z, w)
		{
			(Some(_), None, None, None) =>          Err(A::Error::invalid_length(1, &"Sequence of length 4")),
			(Some(_), Some(_), None, None) =>       Err(A::Error::invalid_length(2, &"Sequence of length 4")),
			(Some(_), Some(_), Some(_), None) =>    Err(A::Error::invalid_length(4, &"Sequence of length 4")),
			(Some(x), Some(y), Some(z), Some(w)) => Ok(Vec4::new(x, y, z, w)),
			_ =>                                    Err(A::Error::custom("Expected array of length 4, found nothing")),
		}
	}
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for QuatVisitor<T>
{
	type Value = Quat<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 4")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let x: Option<T> = a.next_element()?;
		let y: Option<T> = a.next_element()?;
		let z: Option<T> = a.next_element()?;
		let w: Option<T> = a.next_element()?;

		match (x, y, z, w)
		{
			(Some(_), None, None, None) =>          Err(A::Error::invalid_length(1, &"Sequence of length 4")),
			(Some(_), Some(_), None, None) =>       Err(A::Error::invalid_length(2, &"Sequence of length 4")),
			(Some(_), Some(_), Some(_), None) =>    Err(A::Error::invalid_length(4, &"Sequence of length 4")),
			(Some(x), Some(y), Some(z), Some(w)) => Ok(Quat::new(x, y, z, w)),
			_ =>                                    Err(A::Error::custom("Expected array of length 4, found nothing")),
		}
	}
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for DualVisitor<T>
{
	type Value = Dual<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 8")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let x1: Option<T> = a.next_element()?;
		let y1: Option<T> = a.next_element()?;
		let z1: Option<T> = a.next_element()?;
		let w1: Option<T> = a.next_element()?;
		let x2: Option<T> = a.next_element()?;
		let y2: Option<T> = a.next_element()?;
		let z2: Option<T> = a.next_element()?;
		let w2: Option<T> = a.next_element()?;

		if 	x1.is_some() && y1.is_some() && z1.is_some() && w1.is_some() && 
			x2.is_some() && y2.is_some() && z2.is_some() && w2.is_some()
		{
			Ok(Dual::new(
				Quat::new(x1.unwrap(), y1.unwrap(), z1.unwrap(), w1.unwrap()),
				Quat::new(x2.unwrap(), y2.unwrap(), z2.unwrap(), w2.unwrap()))
				)
		}
		else
		{
			Err(A::Error::invalid_length(1, &"Sequence of length 8"))
		}
	}
}


impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for Mat2Visitor<T>
{
	type Value = Mat2<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 4")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let x: Option<T> = a.next_element()?;
		let y: Option<T> = a.next_element()?;
		let z: Option<T> = a.next_element()?;
		let w: Option<T> = a.next_element()?;

		match (x, y, z, w)
		{
			(Some(_), None, None, None) =>          Err(A::Error::invalid_length(1, &"Sequence of length 4")),
			(Some(_), Some(_), None, None) =>       Err(A::Error::invalid_length(2, &"Sequence of length 4")),
			(Some(_), Some(_), Some(_), None) =>    Err(A::Error::invalid_length(4, &"Sequence of length 4")),
			(Some(x), Some(y), Some(z), Some(w)) => Ok(Mat2::new(x, y, z, w)),
			_ =>                                    Err(A::Error::custom("Expected array of length 4, found nothing")),
		}
	}
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for Mat3Visitor<T>
{
	type Value = Mat3<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 9")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let a1: Option<T> = a.next_element()?;
		let a2: Option<T> = a.next_element()?;
		let a3: Option<T> = a.next_element()?;
		let b1: Option<T> = a.next_element()?;
		let b2: Option<T> = a.next_element()?;
		let b3: Option<T> = a.next_element()?;
		let c1: Option<T> = a.next_element()?;
		let c2: Option<T> = a.next_element()?;
		let c3: Option<T> = a.next_element()?;

		if 	a1.is_some() && a2.is_some() && a3.is_some() && 
			b1.is_some() && b2.is_some() && b3.is_some() && 
			c1.is_some() && c2.is_some() && c3.is_some()
		{
			Ok(Mat3::new(
				a1.unwrap(), a2.unwrap(), a3.unwrap(),
				b1.unwrap(), b2.unwrap(), b3.unwrap(),
				c1.unwrap(), c2.unwrap(), c3.unwrap()))
		}
		else
		{
			Err(A::Error::invalid_length(1, &"Sequence of length 9"))
		}
	}
}

impl<'de, T: Deserialize<'de> + Copy> Visitor<'de> for Mat4Visitor<T>
{
	type Value = Mat4<T>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		f.write_str("A sequence of length 16")
	}

	fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>,
	{
		let a1: Option<T> = a.next_element()?;
		let a2: Option<T> = a.next_element()?;
		let a3: Option<T> = a.next_element()?;
		let a4: Option<T> = a.next_element()?;
		let b1: Option<T> = a.next_element()?;
		let b2: Option<T> = a.next_element()?;
		let b3: Option<T> = a.next_element()?;
		let b4: Option<T> = a.next_element()?;
		let c1: Option<T> = a.next_element()?;
		let c2: Option<T> = a.next_element()?;
		let c3: Option<T> = a.next_element()?;
		let c4: Option<T> = a.next_element()?;
		let d1: Option<T> = a.next_element()?;
		let d2: Option<T> = a.next_element()?;
		let d3: Option<T> = a.next_element()?;
		let d4: Option<T> = a.next_element()?;

		if 	a1.is_some() && a2.is_some() && a3.is_some() && a4.is_some() && 
			b1.is_some() && b2.is_some() && b3.is_some() && b4.is_some() && 
			c1.is_some() && c2.is_some() && c3.is_some() && c4.is_some() && 
			d1.is_some() && d2.is_some() && d3.is_some() && d4.is_some()
		{
			Ok(Mat4::new(
				a1.unwrap(), a2.unwrap(), a3.unwrap(), a4.unwrap(),
				b1.unwrap(), b2.unwrap(), b3.unwrap(), b4.unwrap(),
				c1.unwrap(), c2.unwrap(), c3.unwrap(), c4.unwrap(),
				d1.unwrap(), d2.unwrap(), d3.unwrap(), d4.unwrap()))
		}
		else
		{
			Err(A::Error::invalid_length(1, &"Sequence of length 16"))
		}
	}
}

impl<T: Serialize> Serialize for Vec2<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(2)?;
		tuple.serialize_element(&self.x)?;
		tuple.serialize_element(&self.y)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Vec3<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(3)?;
		tuple.serialize_element(&self.x)?;
		tuple.serialize_element(&self.y)?;
		tuple.serialize_element(&self.z)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Vec4<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(4)?;
		tuple.serialize_element(&self.x)?;
		tuple.serialize_element(&self.y)?;
		tuple.serialize_element(&self.z)?;
		tuple.serialize_element(&self.w)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Quat<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(4)?;
		tuple.serialize_element(&self.x)?;
		tuple.serialize_element(&self.y)?;
		tuple.serialize_element(&self.z)?;
		tuple.serialize_element(&self.w)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Dual<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(8)?;
		tuple.serialize_element(&self.real.x)?;
		tuple.serialize_element(&self.real.y)?;
		tuple.serialize_element(&self.real.z)?;
		tuple.serialize_element(&self.real.w)?;
		tuple.serialize_element(&self.dual.x)?;
		tuple.serialize_element(&self.dual.y)?;
		tuple.serialize_element(&self.dual.z)?;
		tuple.serialize_element(&self.dual.w)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Mat2<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(4)?;
		tuple.serialize_element(&self.a1)?;
		tuple.serialize_element(&self.a2)?;
		tuple.serialize_element(&self.b1)?;
		tuple.serialize_element(&self.b2)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Mat3<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(9)?;
		tuple.serialize_element(&self.a1)?;
		tuple.serialize_element(&self.a2)?;
		tuple.serialize_element(&self.a3)?;
		tuple.serialize_element(&self.b1)?;
		tuple.serialize_element(&self.b2)?;
		tuple.serialize_element(&self.b3)?;
		tuple.serialize_element(&self.c1)?;
		tuple.serialize_element(&self.c2)?;
		tuple.serialize_element(&self.c3)?;
		tuple.end()
	}
}

impl<T: Serialize> Serialize for Mat4<T>
{
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>
	{
		let mut tuple = s.serialize_tuple(16)?;
		tuple.serialize_element(&self.a1)?;
		tuple.serialize_element(&self.a2)?;
		tuple.serialize_element(&self.a3)?;
		tuple.serialize_element(&self.a4)?;
		tuple.serialize_element(&self.b1)?;
		tuple.serialize_element(&self.b2)?;
		tuple.serialize_element(&self.b3)?;
		tuple.serialize_element(&self.b4)?;
		tuple.serialize_element(&self.c1)?;
		tuple.serialize_element(&self.c2)?;
		tuple.serialize_element(&self.c3)?;
		tuple.serialize_element(&self.c4)?;
		tuple.serialize_element(&self.d1)?;
		tuple.serialize_element(&self.d2)?;
		tuple.serialize_element(&self.d3)?;
		tuple.serialize_element(&self.d4)?;
		tuple.end()
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Vec2<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(2, Vec2Visitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Vec3<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(3, Vec3Visitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Vec4<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(4, Vec4Visitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Quat<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(4, QuatVisitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Dual<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(8, DualVisitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Mat2<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(4, Mat2Visitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Mat3<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(9, Mat3Visitor::new())
	}
}

impl<'de, T: Deserialize<'de> + Copy> Deserialize<'de> for Mat4<T>
{
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	{
		d.deserialize_tuple(16, Mat4Visitor::new())
	}
}