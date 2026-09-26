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

/// the scalar of the vector and scalar mixed arithmetic must be the component type
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec3<f32>>() + zeroed_val::<u32>();
/// ```
pub struct VectorAddMismatchedScalar;

/// the vector operands must have the same size
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec3<f32>>() / zeroed_val::<Vec2<f32>>();
/// ```
pub struct VectorDivMismatchedSize;

/// matrix and scalar can only be multiplied
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat4<f32>>() + zeroed_val::<f32>();
/// ```
pub struct MatrixAddScalar;

/// matrix does not support division
/// ```compile_fail,E0369
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat4<f32>>() / zeroed_val::<Mat4<f32>>();
/// ```
pub struct MatrixDivMatrix;

/// the result of the compound assignment must be the left operand type
/// ```compile_fail,E0271
/// use rendiation_shader_api::*;
/// let mut s = zeroed_val::<f32>();
/// s += zeroed_val::<Vec3<f32>>();
/// ```
pub struct ScalarAddAssignVector;

/// matCxR can only multiply vecC
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat2x3<f32>>() * zeroed_val::<Vec3<f32>>();
/// ```
pub struct NonSquareMatrixMulVectorMismatch;

/// matCxR can only multiply matNxC
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat3x4<f32>>() * zeroed_val::<Mat3x4<f32>>();
/// ```
pub struct NonSquareMatrixMulMatrixMismatch;

/// vec3 has no w component
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec3<f32>>().xyw();
/// ```
pub struct Vec3SwizzleW;

/// the xyzw and rgba names can not be mixed
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec4<f32>>().xg();
/// ```
pub struct SwizzleMixedNames;

/// the swizzle component index must be less than the vector size
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec3<f32>>().swizzle2::<0, 3>();
/// ```
pub struct SwizzleComponentOutOfRange;

/// the component index must be less than the vector size
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec2<f32>>().component::<2>();
/// ```
pub struct ComponentOutOfRange;
