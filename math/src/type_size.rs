use super::vec2::*;
use super::vec3::*;
use super::vec4::*;
use super::mat2::*;
use super::mat3::*;
use super::mat4::*;

pub trait TypeSize<T> {
	fn type_size() -> usize {
		return std::mem::size_of::<T>();
	}
}

impl TypeSize<i8> for i8 {}
impl TypeSize<i16> for i16 {}
impl TypeSize<i32> for i32 {}
impl TypeSize<i64> for i64 {}
impl TypeSize<isize> for isize {}
impl TypeSize<u8> for u8 {}
impl TypeSize<u16> for u16 {}
impl TypeSize<u32> for u32 {}
impl TypeSize<u64> for u64 {}
impl TypeSize<usize> for usize {}
impl TypeSize<f32> for f32 {}
impl TypeSize<f64> for f64 {}
impl TypeSize<char> for char {}
impl TypeSize<bool> for bool {}
impl TypeSize<()> for () {}
impl<T> TypeSize<Vec2<T>> for Vec2<T> {}
impl<T> TypeSize<Vec3<T>> for Vec3<T> {}
impl<T> TypeSize<Vec4<T>> for Vec4<T> {}
impl<T> TypeSize<Mat2<T>> for Mat2<T> {}
impl<T> TypeSize<Mat3<T>> for Mat3<T> {}
impl<T> TypeSize<Mat4<T>> for Mat4<T> {}