/// saturate only accepts float
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val(1_u32).saturate();
/// ```
pub struct SaturateInteger;

/// WGSL smoothstep has no overload that mixes scalar x with vector edges
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let edge = zeroed_val::<Vec3<f32>>();
/// let v = val(0.5_f32).smoothstep(edge, edge);
/// ```
pub struct SmoothstepMixedScalarVector;

/// the exponent of vector frexp is vector
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// let (_, exp): (_, Node<i32>) = zeroed_val::<Vec3<f32>>().frexp();
/// ```
pub struct FrexpVectorExponentIsVector;

/// bitcast can not change the vector size
/// ```compile_fail,E0271
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec3<f32>>().bitcast::<Vec4<u32>>();
/// ```
pub struct BitcastVectorSizeMismatch;

/// bitcast can not convert vector to scalar
/// ```compile_fail,E0271
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Vec2<f32>>().bitcast::<u32>();
/// ```
pub struct BitcastVectorToScalar;

/// bool can not be bitcast
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val(true).bitcast::<u32>();
/// ```
pub struct BitcastBool;

/// determinant only applies to the square matrix
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat2x3<f32>>().determinant();
/// ```
pub struct NonSquareMatrixDeterminant;

/// the transpose of matCxR is matRxC
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// let m: Node<Mat2x3<f32>> = zeroed_val::<Mat2x3<f32>>().transpose();
/// ```
pub struct NonSquareMatrixTransposeType;
