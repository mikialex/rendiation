pub struct BasicShadowMapSystemInputs {
  ///  alloc_id => shadow camera vp
  pub source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
  /// alloc_id => shadow map resolution
  pub size: Box<dyn ReactiveCollection<u32, Size>>,
  /// alloc_id => shadow map bias
  pub bias: Box<dyn ReactiveCollection<u32, ShadowBias>>,
  /// alloc_id => enabled
  pub bias: Box<dyn ReactiveCollection<u32, bool>>,
}

pub fn basic_shadow_map_uniform(
  inputs: BasicShadowMapSystemInputs,
) -> (AllocatedShadowMap, ClampedUniformList<BasicShadowMapInfo>) {
  todo!()
}

pub fn basic_shadow_map_storage(
  inputs: BasicShadowMapSystemInputs,
) -> (AllocatedShadowMap, StorageBufferView<BasicShadowMapInfo>) {
  todo!()
}

pub fn basic_shadow_map_maintainer(
  source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
  size: Box<dyn ReactiveCollection<u32, Size>>,
) -> (
  AllocatedShadowMap,
  impl ReactiveCollection<u32, ShadowMapAddressInfo>,
) {
  todo!()
}

pub struct AllocatedShadowMap {
  shadow_map_gpu: GPUTexture,
  shadow_map_allocation: Box<dyn ReactiveCollection<u32, ShadowMapAddressInfo>>,
}

impl AllocatedShadowMap {
  pub fn update_shadow_maps(&self, frame_ctx: FrameCtx, scene: PassContent) {
    //
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct BasicShadowMapInfo {
  pub enabled: bool,
  pub shadow_camera_view_proj: Mat4<f32>,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct ShadowBias {
  pub bias: f32,
  pub normal_bias: f32,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct ShadowMapAddressInfo {
  pub layer_index: i32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}
