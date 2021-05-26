pub mod basic;
pub use basic::*;
use rendiation_algebra::Mat4;

use crate::Renderer;

use super::Camera;

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
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}

pub struct MaterialCell<T: MaterialCPUResource> {
  material: T,
  gpu: T::GPU,
}

pub struct SceneMaterialRenderPrepareCtx<'a> {
  pub camera: &'a Camera,
  pub camera_gpu: &'a wgpu::Buffer,
  pub model_matrix: &'a Mat4<f32>,
  pub model_matrix_gpu: &'a wgpu::Buffer,
}

pub trait Material {
  fn update<'a>(&mut self, renderer: &mut Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a>);
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}

impl<T> Material for MaterialCell<T>
where
  T: MaterialCPUResource,
{
  fn update<'a>(&mut self, renderer: &mut Renderer, ctx: &mut SceneMaterialRenderPrepareCtx<'a>) {
    self.gpu.update(&self.material, renderer, ctx);
  }
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    self.gpu.setup_pass(pass)
  }
}
