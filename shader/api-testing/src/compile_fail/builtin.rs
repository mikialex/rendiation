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
