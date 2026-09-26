/// integer texture can not be sampled by sampler, only loaded or gathered
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderTexture2DUint>, s: BindingNode<ShaderSampler>) {
///   tex.sample(s, zeroed_val::<Vec2<f32>>());
/// }
/// ```
pub struct SampleIntegerTexture;

/// comparison sampling requires depth texture
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderTexture2D>, s: BindingNode<ShaderCompareSampler>) {
///   tex.build_compare_sample_call(s, zeroed_val::<Vec2<f32>>(), val(0.5));
/// }
/// ```
pub struct CompareSampleColorTexture;

/// depth texture does not support bias sampling
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderDepthTexture2D>, s: BindingNode<ShaderSampler>) {
///   let call = tex.build_sample_call(s, zeroed_val::<Vec2<f32>>());
///   call.with_level_bias(val(1.));
/// }
/// ```
pub struct BiasSampleDepthTexture;

/// the explicit level of depth texture is integer
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderDepthTexture2D>, s: BindingNode<ShaderSampler>) {
///   let call = tex.build_sample_call(s, zeroed_val::<Vec2<f32>>());
///   call.with_level(val(1.0_f32));
/// }
/// ```
pub struct FloatLevelDepthTexture;

/// cube texture does not support sampling offset
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderTextureCube>, s: BindingNode<ShaderSampler>) {
///   let call = tex.build_sample_call(s, zeroed_val::<Vec3<f32>>());
///   call.with_offset(Vec2::new(1, 1));
/// }
/// ```
pub struct CubeTextureOffset;

/// the result of textureGatherCompare is always vec4<f32>
/// ```compile_fail,E0308
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderDepthTexture2D>, s: BindingNode<ShaderCompareSampler>) {
///   let call = tex.build_compare_sample_call(s, zeroed_val::<Vec2<f32>>(), val(0.5));
///   let v: Node<Vec4<u32>> = call.gather();
/// }
/// ```
pub struct GatherCompareResultType;

/// storage texture has no mip level
/// ```compile_fail,E0061
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderStorageTextureR2D>) {
///   tex.texture_dimension_2d(Some(val(0)));
/// }
/// ```
pub struct StorageTextureDimensionLevel;
