
use crate::vec4::Vec4;
use crate::mat4::Mat4;

impl Mat4<f32> {
	pub fn max_scale_on_axis(&self) -> f32
	{
		let scale_x_sq = self.a1 * self.a1 + self.a2 * self.a2 + self.a3 * self.a3;
		let scale_y_sq = self.b1 * self.b1 + self.b2 * self.b2 + self.b3 * self.b3;
		let scale_z_sq = self.c1 * self.c1 + self.c2 * self.c2 + self.c3 * self.c3;

		scale_x_sq.max(scale_y_sq).max(scale_z_sq).sqrt()
	}
	
	pub const fn const_new(
		m11:f32, m12:f32, m13:f32, m14:f32, 
		m21:f32, m22:f32, m23:f32, m24:f32, 
		m31:f32, m32:f32, m33:f32, m34:f32, 
		m41:f32, m42:f32, m43:f32, m44:f32) -> Self
	{
		Self
		{ 
			a1:m11, a2:m12, a3:m13, a4:m14,
			b1:m21, b2:m22, b3:m23, b4:m24,
			c1:m31, c2:m32, c3:m33, c4:m34,
			d1:m41, d2:m42, d3:m43, d4:m44,
		}
	}
}

use std::mem;

impl<T> AsRef<[T; 16]> for Mat4<T> {
	#[inline]
	fn as_ref(&self) -> &[T; 16] {
		unsafe { mem::transmute(self) }
	}
}

impl<T> AsRef<[T; 4]> for Vec4<T> {
	#[inline]
	fn as_ref(&self) -> &[T; 4] {
		unsafe { mem::transmute(self) }
	}
}
