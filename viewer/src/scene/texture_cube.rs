use rendiation_webgpu::*;

pub struct SceneTextureCube {
  data: [Box<dyn SceneTexture2dSource>; 6],
  gpu: Option<SceneTextureCubeGPU>,
}
