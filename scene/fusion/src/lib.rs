use rendiation_scene_core::SceneContent;
use rendiation_scene_raytracing::*;
use rendiation_scene_webgpu::*;
use rendiation_webgpu::*;

#[derive(Copy, Clone)]
pub struct FusionScene;
impl SceneContent for FusionScene {
  type BackGround = Box<dyn FusionBackground>;
  type Model = Box<dyn FusionModel>;
  type Light = Box<dyn SceneRenderableShareable>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
  type SceneExt = ();
}

pub trait FusionBackground: RayTracingBackground + WebGPUBackground {}

pub trait FusionModel: RayTracingModel + SceneRenderableShareable {}

pub trait FusionSceneExtension {
  fn add_model(&mut self, model: impl FusionModel);
}

impl FusionSceneExtension for Scene<FusionScene> {
  fn add_model(&mut self, model: impl FusionModel) {
    self.models.push(Box::new(model));
  }
}
