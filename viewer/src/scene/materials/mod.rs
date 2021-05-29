pub mod basic;
pub use basic::*;
use rendiation_algebra::Mat4;

use crate::Renderer;

use super::{Camera, CameraBindgroup, RenderStyle};

pub trait MaterialRenderStyleSupport<S: RenderStyle> {
  type Source;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn setup_pass<'a>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
  );
}

pub trait MaterialCPUResource {
  type GPU;
  fn create(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource<S>: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn setup_pass<'a>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
    style: &S,
  );
}

pub struct MaterialCell<T>
where
  T: MaterialCPUResource,
{
  material: T,
  gpu: T::GPU,
}

pub struct SceneMaterialRenderPrepareCtx<'a> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub pipelines: &'a mut PipelineResourceManager,
}

pub trait Material<S: RenderStyle> {
  fn update<'a>(
    &mut self,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<'a>,
    style: &S,
  );
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
    style: &'a S,
  );
}

impl<T, S> Material<S> for MaterialCell<T>
where
  T: MaterialCPUResource,
  S: RenderStyle,
  <T as MaterialCPUResource>::GPU: MaterialGPUResource<S, Source = T>,
{
  fn update<'a>(
    &mut self,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<'a>,
    style: &S,
  ) {
    self.gpu.update(&self.material, renderer, ctx);
  }
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
    style: &'a S,
  ) {
    self.gpu.setup_pass(pass, pipeline_manager, style)
  }
}

pub struct PipelineResourceManager {
  basic: Option<wgpu::RenderPipeline>,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self { basic: None }
  }
}
