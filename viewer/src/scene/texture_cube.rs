use super::{BindableResource, SceneTexture2dSource};

pub struct SceneTextureCube {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTextureCubeGPU>,
}
