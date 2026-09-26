/// the user defined IO can not be bool
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// both!(Flag, bool);
/// fn case(builder: &mut ShaderVertexBuilder) {
///   builder.set_vertex_out::<Flag>(val(true));
/// }
/// ```
pub struct VertexOutBool;

/// the user defined IO can not be matrix
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// both!(Transform, Mat4<f32>);
/// fn case(builder: &mut ShaderVertexBuilder) {
///   builder.set_vertex_out::<Transform>(zeroed_val::<Mat4<f32>>());
/// }
/// ```
pub struct VertexOutMatrix;

/// the vertex input can not be bool
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// only_vertex!(Flag, bool);
/// fn case(builder: &mut ShaderRawVertexBuilder) {
///   builder.register_vertex_in::<Flag>();
/// }
/// ```
pub struct VertexInBool;
