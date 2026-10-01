/// the uniform buffer requires the std140 layout
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// use rendiation_webgpu::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: f32,
/// }
/// fn case(_: UniformBufferDataView<Case>) {}
/// ```
pub struct UniformOfStd430Struct;

/// the storage buffer requires the std430 layout
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// use rendiation_webgpu::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: f32,
/// }
/// fn case(_: StorageBufferReadonlyDataView<Case>) {}
/// ```
pub struct StorageOfStd140Struct;

/// the readonly storage buffer item can not be written
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(p: ShaderReadonlyPtrOf<[u32]>) {
///   p.index(val(0_u32)).store(val(1_u32));
/// }
/// ```
pub struct ReadonlyStorageStore;

/// the uniform matrix column can not be written
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(p: ShaderReadonlyPtrOf<Mat4<f32>>) {
///   p.x().store(zeroed_val());
/// }
/// ```
pub struct UniformMatrixColumnStore;

/// the workgroup uniform load requires the writable pointer of the workgroup variable
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// fn case(p: ShaderReadonlyPtrOf<f32>) {
///   workgroup_uniform_load::<f32>(p);
/// }
/// ```
pub struct WorkgroupUniformLoadReadonly;
