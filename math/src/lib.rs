
#![feature(specialization)]
#![feature(const_generics)]
#![feature(never_type)]
#![allow(incomplete_features)]

pub mod vec;
pub mod mat;
pub mod space;
pub mod math;
pub mod utils;
pub mod quat;
pub mod dual;
pub mod consts;
pub mod swizzle;
pub mod interpolation;

pub mod wasm;
pub use wasm::*;

#[macro_use]
pub mod marcos;
pub use space::*;
pub use vec::*;
pub use mat::*;
pub use math::*;
pub use interpolation::*;

pub use self::quat::*;
pub use self::dual::*;
pub use self::consts::*;

// pub mod ser;
// pub use self::ser::*;


#[macro_export] macro_rules! float 
{ 
	($x:expr,$y:expr) => { float2::new($x,$y) };
	($x:expr,$y:expr,$z:expr) => { float3::new($x,$y,$z) };
	($x:expr,$y:expr,$z:expr,$w:expr) => { float4::new($x,$y,$z,$w) };
	($m11:expr, $m12:expr, $m13:expr, 
	 $m21:expr, $m22:expr, $m23:expr, 
	 $m31:expr, $m32:expr, $m33:expr) =>
	{
		float3x3::new(
			$m11, $m12, $m13,
			$m21, $m22, $m23,
			$m31, $m32, $m33
		)
	};
	($m11:expr, $m12:expr, $m13:expr, $m14:expr, 
	 $m21:expr, $m22:expr, $m23:expr, $m24:expr, 
	 $m31:expr, $m32:expr, $m33:expr, $m34:expr,
	 $m41:expr, $m42:expr, $m43:expr, $m44:expr) =>
	{
		float4x4::new(
			$m11, $m12, $m13, $m14,
			$m21, $m22, $m23, $m24,
			$m31, $m32, $m33, $m34,
			$m41, $m42, $m43, $m44,
		)
	};
}