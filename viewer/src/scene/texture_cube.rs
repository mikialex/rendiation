use rendiation_webgpu::*;

pub struct SceneTextureCube {
  data: [Box<dyn WebGPUTexture2dSource>; 6],
  gpu: Option<WebGPUTextureCube>,
}
