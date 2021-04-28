pub mod basic;
pub use basic::*;

use crate::{Material, Renderer};

pub trait MaterialCPUResource {
  type GPU: MaterialGPUResource;
  fn create(&mut self, renderer: &mut Renderer) -> Self::GPU;
}

pub trait MaterialGPUResource {
  type Source;
  fn update(&mut self, source: &Self::Source, renderer: &mut Renderer);
}

pub struct MaterialCell<T: MaterialCPUResource> {
  material: T,
  gpu: T::GPU,
}

pub struct SceneMaterialRenderCtx {}

impl<T: MaterialCPUResource> Material for MaterialCell<T> {
  fn update(&mut self, renderer: &mut Renderer, des: &wgpu::RenderPassDescriptor) {
    // self.material.update(renderer, des);
  }
  fn setup_pass<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
    ctx: &mut SceneMaterialRenderCtx,
  ) {
  }
}
