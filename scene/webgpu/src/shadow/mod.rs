use crate::*;

type LightId = u64;

pub struct ShadowMapAllocator {
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

pub struct ShadowMapAllocatorImpl {
  gpu: GPUTexture2d,
  mapping: HashMap<LightId, GPUTexture2dView>,
}

pub struct ShadowMap {
  layer: LightId,
  inner: Rc<RefCell<ShadowMapAllocatorImpl>>,
}

impl Drop for ShadowMap {
  fn drop(&mut self) {
    todo!()
  }
}

impl ShadowMap {
  pub fn is_content_lost(&self) -> bool {
    todo!()
  }

  pub fn get_view(&self, gpu: &GPU) -> GPUTexture2dView {
    todo!()
  }
}

impl ShadowMapAllocator {
  pub fn with_capacity(size: Size, layer: usize, gpu: &GPU) -> Self {
    todo!()
  }

  pub fn allocate(&self, gpu: &GPU, light: LightId, resolution: Size) -> ShadowMap {
    todo!()
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct ShadowMappingInfo {
  pub shadow_camera: CameraGPUTransform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct ShadowBias {
  pub bias: f32,
  pub normal_bias: f32,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct ShadowMapAddressInfo {
  pub layer_index: f32,
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}
