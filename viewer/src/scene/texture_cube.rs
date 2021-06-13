use crate::renderer::SceneTextureCubeGPU;

use super::SceneTexture2dSource;

pub struct SceneTextureCube {
  data: [Box<dyn SceneTexture2dSource>; 6],
  gpu: Option<SceneTextureCubeGPU>,
}
