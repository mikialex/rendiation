use arena::Handle;
use rendiation_scene_core::SceneContent;
use rendiation_scene_raytracing::*;
pub use rendiation_scene_webgpu::*;
use rendiation_webgpu::*;

#[derive(Copy, Clone)]
pub struct FusionScene;
impl SceneContent for FusionScene {
  type BackGround = Box<dyn FusionBackground>;
  type Model = Box<dyn FusionModel>;
  type Light = Box<dyn FusionLight>;
  type Texture2D = Box<dyn WebGPUTexture2dSource>;
  type TextureCube = [Box<dyn WebGPUTexture2dSource>; 6];
  type SceneExt = ();
}

pub trait FusionBackground: RayTracingBackground + WebGPUBackground {}
impl<T: RayTracingBackground + WebGPUBackground> FusionBackground for T {}

pub trait FusionModel: RayTracingModel + SceneRenderableShareable + 'static {}
impl<T: RayTracingModel + SceneRenderableShareable + 'static> FusionModel for T {}

pub trait FusionLight: WebGPUSceneLight {}
impl<T: WebGPUSceneLight> FusionLight for T {}

pub type FusionModelHandle = Handle<Box<dyn FusionModel>>;

pub trait FusionSceneExtension {
  #[must_use]
  fn add_model(&mut self, model: impl FusionModel) -> FusionModelHandle;
  fn remove_model(&mut self, handle: FusionModelHandle) -> bool;
}

impl FusionSceneExtension for Scene<FusionScene> {
  fn add_model(&mut self, model: impl FusionModel) -> FusionModelHandle {
    self.models.insert(Box::new(model))
  }
  fn remove_model(&mut self, handle: FusionModelHandle) -> bool {
    self.models.remove(handle).is_some()
  }
}

impl WebGPUBackground for Box<dyn FusionBackground> {
  fn require_pass_clear(&self) -> Option<rendiation_webgpu::Color> {
    todo!()
  }
}

impl SceneRenderable for Box<dyn FusionBackground> {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    todo!()
  }
}

impl WebGPUSceneLight for Box<dyn FusionLight> {
  fn collect(&self, res: &mut ForwardLightingSystem) {
    todo!()
  }
}
