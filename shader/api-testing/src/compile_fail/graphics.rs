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

/// the clip distances count must be in [1, 8]
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// fn case(builder: &mut ShaderRawVertexBuilder) {
///   builder.set_clip_distances(zeroed_val::<[f32; 9]>());
/// }
/// let _: fn(&mut ShaderRawVertexBuilder) = case;
/// ```
pub struct ClipDistancesTooMany;

/// the clip distances can not be empty
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// fn case(builder: &mut ShaderRawVertexBuilder) {
///   builder.set_clip_distances(zeroed_val::<[f32; 0]>());
/// }
/// let _: fn(&mut ShaderRawVertexBuilder) = case;
/// ```
pub struct ClipDistancesEmpty;

/// the inter stage IO semantic must be readable in the fragment stage
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// only_vertex!(VertexOnly, f32);
/// fn case(builder: &mut ShaderVertexBuilder) {
///   builder.set_vertex_out::<VertexOnly>(val(1.));
/// }
/// ```
pub struct VertexOutVertexOnlySemantic;

/// the depth output is f32
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(builder: &mut ShaderFragmentBuilder) {
///   builder.register::<FragmentDepthOutput>(val(1_u32));
/// }
/// ```
pub struct DepthOutputNotF32;

/// the early depth test is a fragment stage config
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(builder: &mut ShaderVertexBuilder) {
///   builder.set_early_depth_test(ShaderEarlyDepthTest::Force);
/// }
/// ```
pub struct EarlyDepthTestInVertex;

/// the mesh task size is vec3<u32>
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// fn case(builder: &mut ShaderTaskBuilder) {
///   builder.set_output_mesh_task_size(zeroed_val::<Vec3<i32>>());
/// }
/// ```
pub struct MeshTaskSizeNotU32;
