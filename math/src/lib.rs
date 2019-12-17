pub mod vec;
pub mod vec2;
pub mod vec3;
pub mod vec4;
pub mod mat2;
pub mod mat3;
pub mod mat4;
pub mod quat;
pub mod dual;
pub mod type_size;
pub mod consts;
// pub mod ser;

pub use self::vec2::*;
pub use self::vec3::*;
pub use self::vec4::*;
pub use self::mat2::*;
pub use self::mat3::*;
pub use self::mat4::*;
pub use self::quat::*;
pub use self::dual::*;
pub use self::consts::*;
// pub use self::ser::*;

#[allow(non_camel_case_types)] pub type i8_1 = i8;
#[allow(non_camel_case_types)] pub type i8_2 = Vec2<i8>;
#[allow(non_camel_case_types)] pub type i8_3 = Vec3<i8>;
#[allow(non_camel_case_types)] pub type i8_4 = Vec4<i8>;

#[allow(non_camel_case_types)] pub type i16_1 = i16;
#[allow(non_camel_case_types)] pub type i16_2 = Vec2<i16>;
#[allow(non_camel_case_types)] pub type i16_3 = Vec3<i16>;
#[allow(non_camel_case_types)] pub type i16_4 = Vec4<i16>;

#[allow(non_camel_case_types)] pub type i32_1 = i32;
#[allow(non_camel_case_types)] pub type i32_2 = Vec2<i32>;
#[allow(non_camel_case_types)] pub type i32_3 = Vec3<i32>;
#[allow(non_camel_case_types)] pub type i32_4 = Vec4<i32>;

#[allow(non_camel_case_types)] pub type u8_1 = u8;
#[allow(non_camel_case_types)] pub type u8_2 = Vec2<u8>;
#[allow(non_camel_case_types)] pub type u8_3 = Vec3<u8>;
#[allow(non_camel_case_types)] pub type u8_4 = Vec4<u8>;

#[allow(non_camel_case_types)] pub type u16_1 = u16;
#[allow(non_camel_case_types)] pub type u16_2 = Vec2<u16>;
#[allow(non_camel_case_types)] pub type u16_3 = Vec3<u16>;
#[allow(non_camel_case_types)] pub type u16_4 = Vec4<u16>;

#[allow(non_camel_case_types)] pub type u32_1 = u32;
#[allow(non_camel_case_types)] pub type u32_2 = Vec2<u32>;
#[allow(non_camel_case_types)] pub type u32_3 = Vec3<u32>;
#[allow(non_camel_case_types)] pub type u32_4 = Vec4<u32>;

#[allow(non_camel_case_types)] pub type float1 = f32;
#[allow(non_camel_case_types)] pub type float2 = Vec2<f32>;
#[allow(non_camel_case_types)] pub type float3 = Vec3<f32>;
#[allow(non_camel_case_types)] pub type float4 = Vec4<f32>;

#[allow(non_camel_case_types)] pub type double1 = f64;
#[allow(non_camel_case_types)] pub type double2 = Vec2<f64>;
#[allow(non_camel_case_types)] pub type double3 = Vec3<f64>;
#[allow(non_camel_case_types)] pub type double4 = Vec4<f64>;

#[allow(non_camel_case_types)] pub type float2x2 = Mat2<f32>;
#[allow(non_camel_case_types)] pub type float3x3 = Mat3<f32>;
#[allow(non_camel_case_types)] pub type float4x4 = Mat4<f32>;

#[allow(non_camel_case_types)] pub type double2x2 = Mat2<f64>;
#[allow(non_camel_case_types)] pub type double3x3 = Mat3<f64>;
#[allow(non_camel_case_types)] pub type double4x4 = Mat4<f64>;

#[allow(non_camel_case_types)] pub type Quaternion = Quat<f32>;
#[allow(non_camel_case_types)] pub type DualQuaternion = Dual<f32>;

#[allow(non_camel_case_types)] pub type float1s = Vec<f32>;
#[allow(non_camel_case_types)] pub type float2s = Vec<Vec2<f32>>;
#[allow(non_camel_case_types)] pub type float3s = Vec<Vec3<f32>>;
#[allow(non_camel_case_types)] pub type float4s = Vec<Vec4<f32>>;

#[allow(non_camel_case_types)] pub type double1s = Vec<f64>;
#[allow(non_camel_case_types)] pub type double2s = Vec<Vec2<f64>>;
#[allow(non_camel_case_types)] pub type double3s = Vec<Vec3<f64>>;
#[allow(non_camel_case_types)] pub type double4s = Vec<Vec4<f64>>;

#[allow(non_camel_case_types)] pub type float2x2s = Vec<Mat2<f32>>;
#[allow(non_camel_case_types)] pub type float3x3s = Vec<Mat3<f32>>;
#[allow(non_camel_case_types)] pub type float4x4s = Vec<Mat4<f32>>;

#[allow(non_camel_case_types)] pub type double2x2s = Vec<Mat2<f64>>;
#[allow(non_camel_case_types)] pub type double3x3s = Vec<Mat3<f64>>;
#[allow(non_camel_case_types)] pub type double4x4s = Vec<Mat4<f64>>;

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