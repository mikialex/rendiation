pub mod basic;
use std::any::Any;

use arena::Handle;
pub use basic::*;

use crate::Renderer;

use super::{MaterialBindableResource, SceneResource};

pub trait MaterialCPUResource {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Any + Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    res: &mut MaterialBindableResource,
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
  fn update(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    res: &mut SceneResource,
  );
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>);
}

impl<T> Material for MaterialCell<T>
where
  T: MaterialCPUResource,
{
  fn update(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    res: &mut SceneResource,
  ) {
    let gpu = res
      .material_gpu
      .get_mut(self.gpu)
      .unwrap()
      .downcast_mut::<T::GPU>()
      .unwrap();
    gpu.update(&self.material, renderer, ctx, &mut res.material_bindable);
  }
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>) {}
}
