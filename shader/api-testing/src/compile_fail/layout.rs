/// the WGSL array stride of the std140 array element must be a multiple of 16, f32 is rejected
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: Shader140Array<f32, 4>,
/// }
/// ```
pub struct Std140ScalarArray;

/// the std140 fixed size array must use Shader140Array
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: [Vec4<f32>; 2],
/// }
/// ```
pub struct Std140RustArray;

/// mat2x2 is not host shareable in std140
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub m: Mat2<f32>,
/// }
/// ```
pub struct Std140Mat2;

/// the std430 struct can not be nested in the std140 struct
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Inner {
///   pub a: f32,
/// }
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub inner: Inner,
/// }
/// ```
pub struct Std430StructInStd140;

/// the std140 struct can not be nested in the std430 struct
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Inner {
///   pub a: f32,
/// }
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub inner: Inner,
/// }
/// ```
pub struct Std140StructInStd430;

/// the rust stride of vec3 array is 12 but the WGSL stride is 16
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: [Vec3<f32>; 2],
/// }
/// ```
pub struct Std430Vec3Array;

/// the rust column stride of mat3x3 is 12 but the WGSL one is 16, use Shader16PaddedMat3
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub m: Mat3<f32>,
/// }
/// ```
pub struct Std430Mat3;

/// bool is not host shareable, use Bool
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub b: bool,
/// }
/// ```
pub struct Std430RustBool;

/// the rust stride of the vec3 runtime sized array is 12 but the WGSL stride is 16
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// use rendiation_webgpu::*;
/// fn case(gpu: &GPU, data: &[Vec3<f32>]) {
///   create_gpu_readonly_storage(data, gpu, "");
/// }
/// let _: fn(&GPU, &[Vec3<f32>]) = case;
/// ```
pub struct Std430Vec3RuntimeArray;
