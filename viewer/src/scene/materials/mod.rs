pub mod basic;

use arena::Arena;
pub use basic::*;
use rendiation_algebra::Mat4;

use crate::Renderer;

use super::{Camera, CameraBindgroup, Material, MaterialHandle, ModelTransformGPU, RenderStyle, Scene, SceneMesh, SceneSampler, SceneTexture2D, WatchedArena};

impl Scene {
  pub fn add_material(&mut self, material: impl Material) -> MaterialHandle {
    self.materials.insert(Box::new(material))
  }
}

pub trait MaterialCPUResource {
  type GPU;
  fn create<S>(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource<S>: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) {
    // default do nothing
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  ) {
    // default do nothing
  }
}

pub struct MaterialCell<T>
where
  T: MaterialCPUResource,
{
  material: T,
  gpu: Option<T::GPU>,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T) -> Self {
    Self {
      material,
      gpu: None,
    }
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a, S> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub model_gpu: &'a ModelTransformGPU,
  pub pipelines: &'a mut PipelineResourceManager,
  pub style: &'a S,
  pub active_mesh: &'a SceneMesh,
  pub textures: &'a mut Arena<SceneTexture2D>,
  pub samplers: &'a mut WatchedArena<SceneSampler>,
}

pub struct SceneMaterialPassSetupCtx<'a, S> {
  pub pipelines: &'a PipelineResourceManager,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a ModelTransformGPU,
  pub style: &'a S,
}

pub trait MaterialStyleAbility<S: RenderStyle> {
  fn update<'a>(&mut self, renderer: &mut Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a, S>);
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  );
}

impl<T, S> MaterialStyleAbility<S> for MaterialCell<T>
where
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<S, Source = T>,
  S: RenderStyle,
{
  fn update<'a>(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<'a, S>,
  ) {
    self
      .gpu
      .get_or_insert_with(|| T::create(&mut self.material, renderer, ctx))
      .update(&self.material, renderer, ctx);
  }
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, S>,
  ) {
    self.gpu.as_ref().unwrap().setup_pass(pass, ctx)
  }
}

pub struct PipelineResourceManager {
  pub basic: Option<wgpu::RenderPipeline>,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self { basic: None }
  }
}
