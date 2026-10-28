/// f16 and f32 can not be mixed, the conversion must be explicit
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = val_f16(1.) + val(1_f32);
/// ```
pub struct MixF16AndF32;

/// the f16 matrix can not be multiplied by the f32 vector
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v = zeroed_val::<Mat2<f16>>() * zeroed_val::<Vec2<f32>>();
/// ```
pub struct F16MatrixMulF32Vector;

/// the derivative functions only accept f32 in WGSL
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val_f16(1.).dpdx();
/// ```
pub struct F16Derivative;

/// quantize_to_f16 only accepts f32
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val_f16(1.).quantize_to_f16();
/// ```
pub struct F16QuantizeToF16;

/// naga IR can not bitcast f16, use pack2x_f16 for the vec2 instead
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val_f16(1.).bitcast::<u32>();
/// ```
pub struct F16Bitcast;

/// the atomic type can not be f16
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// let v: ShaderPtrOf<DeviceAtomic<f16>> = make_local_var();
/// ```
pub struct F16Atomic;

/// WGSL has no f16 texture
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// <ShaderTexture<TextureDimension2, f16> as ShaderNodeType>::ty();
/// ```
pub struct F16Texture;

/// the f16 matrix is not supported in std140
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub m: Mat4<f16>,
/// }
/// ```
pub struct Std140F16Matrix;

/// the f16 matCx3 is not host shareable, the rust column stride is 6 but 8 in WGSL
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub m: Mat3<f16>,
/// }
/// ```
pub struct Std430F16Mat3;

/// the rust array stride of vec3<f16> is 6 but 8 in WGSL
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std430)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: [Vec3<f16>; 4],
/// }
/// ```
pub struct Std430F16Vec3Array;

/// the WGSL array stride of vec4<f16> is 8, not a multiple of 16
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// #[repr(C)]
/// #[shader_struct(std140)]
/// #[derive(Clone, Copy)]
/// pub struct Case {
///   pub a: Shader140Array<Vec4<f16>, 4>,
/// }
/// ```
pub struct Std140F16Vec4Array;

/// there is no 3 components f16 vertex format
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case<T: VertexInShaderNodeType>() {}
/// case::<Vec3<f16>>();
/// ```
pub struct F16Vec3VertexInput;

/// the vertex struct fields are tightly packed attributes, the rust padding before the f32 is
/// rejected
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// both!(A, f16);
/// both!(B, f32);
/// #[repr(C)]
/// #[derive(Clone, Copy, ShaderVertex)]
/// pub struct Case {
///   #[semantic(A)]
///   pub a: f16,
///   #[semantic(B)]
///   pub b: f32,
/// }
/// ```
pub struct VertexStructPadding;

/// the uniform buffer size must be a multiple of 4 to be written by the queue
/// ```compile_fail,E0080
/// use rendiation_shader_api::*;
/// use rendiation_webgpu::*;
/// fn case(device: &GPUDevice) {
///   create_uniform(Vec3::<f16>::default(), device, "case");
/// }
/// let case: fn(&GPUDevice) = case;
/// ```
pub struct F16UniformSize;
