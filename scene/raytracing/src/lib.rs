#[derive(Copy, Clone)]
pub struct RayTracingScene;
impl SceneContent for RayTracingScene {
  type BackGround = Box<dyn WebGPUBackground>;
  type Model = Box<dyn SceneRenderableShareable>;
  type Light = Box<dyn SceneRenderableShareable>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
}
