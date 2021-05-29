pub mod basic;
pub use basic::*;
use rendiation_algebra::Mat4;

use crate::Renderer;

use super::{Camera, CameraBindgroup};

// pub trait MaterialRenderable<PassSchema, Vertex>{

// }

pub trait MaterialCPUResource {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn setup_bindgroup<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
  fn setup_pipeline<'a>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline_manager: &'a PipelineResourceManager,
  );
}

pub struct MaterialCell<T: MaterialCPUResource> {
  material: T,
  gpu: T::GPU,
}

pub struct SceneMaterialRenderPrepareCtx<'a> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub pipelines: &'a mut PipelineResourceManager,
}

pub trait Material {
  fn update<'a>(&mut self, renderer: &Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a>);
  fn setup_bindgroup<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}

impl<T> Material for MaterialCell<T>
where
  T: MaterialCPUResource,
{
  fn update<'a>(&mut self, renderer: &Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a>) {
    self.gpu.update(&self.material, renderer, ctx);
  }
  fn setup_bindgroup<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    self.gpu.setup_bindgroup(pass)
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
