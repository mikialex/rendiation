/// the components of a writable swizzle view must be distinct
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(v: ShaderPtrOf<Vec2<f32>>) {
///   v.xx().store(zeroed_val());
/// }
/// ```
pub struct SwizzleViewRepeatedComponents;

/// the components of a writable swizzle view must be distinct
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// fn case(v: ShaderPtrOf<Vec4<f32>>) {
///   v.swizzle3::<0, 1, 0>();
/// }
/// let _: fn(ShaderPtrOf<Vec4<f32>>) = case;
/// ```
pub struct SwizzleViewRepeatedComponentIndices;

/// the swizzle view requires the read_write access, load the vector and swizzle instead
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(v: ShaderReadonlyPtrOf<Vec4<f32>>) {
///   v.xy();
/// }
/// ```
pub struct ReadonlySwizzleView;

/// vec3 has no w component
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(v: ShaderPtrOf<Vec3<f32>>) {
///   v.w();
/// }
/// ```
pub struct Vec3ComponentReferenceW;

/// mat2x3 has two columns
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(m: ShaderPtrOf<Mat2x3<f32>>) {
///   m.z();
/// }
/// ```
pub struct Mat2x3ColumnReferenceZ;
