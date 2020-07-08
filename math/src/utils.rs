
use crate::*;
use std::mem;

impl Mat4<f32> {
	pub fn max_scale_on_axis(&self) -> f32
	{
		let scale_x_sq = self.a1 * self.a1 + self.a2 * self.a2 + self.a3 * self.a3;
		let scale_y_sq = self.b1 * self.b1 + self.b2 * self.b2 + self.b3 * self.b3;
		let scale_z_sq = self.c1 * self.c1 + self.c2 * self.c2 + self.c3 * self.c3;

		scale_x_sq.max(scale_y_sq).max(scale_z_sq).sqrt()
	}
}


impl AsRef<[u8]> for Mat4<f32> {
	#[inline]
	fn as_ref(&self) -> &[u8] {
		unsafe { mem::transmute::<&Mat4<f32>, &[u8; 16 * 4]>(self) }
	}
}

impl AsRef<[u8]> for Vec3<f32> {
	#[inline]
	fn as_ref(&self) -> &[u8] {
		unsafe { mem::transmute::<&Vec3<f32>, &[u8; 3 * 4]>(self) }
	}
}
