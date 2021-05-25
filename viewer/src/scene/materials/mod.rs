pub mod basic;
use std::any::Any;

use arena::Handle;
pub use basic::*;

use crate::Renderer;

pub trait MaterialCPUResource {
  type GPU: MaterialGPUResource;
  fn create(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Any + Sized {
  type Source;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn into_any(self) -> Box<dyn Any> {
    Box::new(self)
  }
}

pub struct MaterialCell<T: MaterialCPUResource> {
  material: T,
  gpu: Handle<Box<dyn Any>>,
}

pub struct SceneMaterialRenderPrepareCtx {
  pub camera: wgpu::Buffer,
}

pub trait Material {
  fn update(&mut self, renderer: &mut Renderer, ctx: &mut SceneMaterialRenderPrepareCtx);
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>);
}

impl<T: MaterialCPUResource> Material for MaterialCell<T> {
  fn update(&mut self, renderer: &mut Renderer, ctx: &mut SceneMaterialRenderPrepareCtx) {
    // self.material.update(renderer, des);
  }
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>) {}
}
