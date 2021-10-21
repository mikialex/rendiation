use std::{
  ops::{Deref, DerefMut},
  rc::Rc,
};

pub struct GPURenderPass<'a> {
  pub(crate) pass: wgpu::RenderPass<'a>,
  pub(crate) holder: &'a GPURenderPassDataHolder,
}

impl<'a> Deref for GPURenderPass<'a> {
  type Target = wgpu::RenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a> DerefMut for GPURenderPass<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

use typed_arena::Arena;

#[derive(Default)]
pub struct GPURenderPassDataHolder {
  buffers: Arena<Rc<wgpu::Buffer>>,
  bindgroup: Arena<Rc<wgpu::BindGroup>>,
  pipelines: Arena<Rc<wgpu::RenderPipeline>>,
}

impl<'a> GPURenderPass<'a> {
  pub fn set_pipeline_owned(&mut self, pipeline: Rc<wgpu::RenderPipeline>) {
    let pipeline = self.holder.pipelines.alloc(pipeline);
    self.pass.set_pipeline(pipeline)
  }
}
