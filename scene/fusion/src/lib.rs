use rendiation_scene_core::SceneContent;
use rendiation_scene_webgpu::*;
use rendiation_webgpu::*;

#[derive(Copy, Clone)]
pub struct FusionScene;
impl SceneContent for FusionScene {
  type BackGround = Box<dyn FusionBackground>;
  type Model = Box<dyn SceneRenderableShareable>;
  type Light = Box<dyn SceneRenderableShareable>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
  type SceneExt = ();
}

pub trait FusionBackground:
  rainray::Background + rendiation_scene_webgpu::WebGPUBackground
{
}
