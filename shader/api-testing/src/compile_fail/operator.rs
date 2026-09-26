/// matrix element type can only be float
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let m: Node<Mat4<u32>> = zeroed_val();
/// ```
pub struct MatrixOfInteger;

/// matrix element type can only be float
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let m: Node<Mat3<bool>> = zeroed_val();
/// ```
pub struct MatrixOfBool;

/// negation only applies to i32 and f32 scalar or vector
/// ```compile_fail,E0600
/// use rendiation_shader_api::*;
/// let v = -val(1_u32);
/// ```
pub struct NegateUnsigned;

/// negation only applies to i32 and f32 scalar or vector
/// ```compile_fail,E0600
/// use rendiation_shader_api::*;
/// let v = -val(true);
/// ```
pub struct NegateBool;

/// the homogeneous transform in math library is not a WGSL operation
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat4<f32>>() * zeroed_val::<Vec3<f32>>();
/// ```
pub struct Mat4MulVec3;

/// the homogeneous transform in math library is not a WGSL operation
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat3<f32>>() * zeroed_val::<Vec2<f32>>();
/// ```
pub struct Mat3MulVec2;

/// mat4x3 * vec4 is vec3 in WGSL
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// let v: Node<Vec4<f32>> = zeroed_val::<Mat4x3<f32>>() * zeroed_val::<Vec4<f32>>();
/// ```
pub struct Mat4x3MulVec4IsVec3;
