use crate::*;

#[derive(Clone)]
pub struct GPURenderPipeline {
  pub inner: Rc<GPURenderPipelineInner>,
}

pub struct GPURenderPipelineInner {
  pub pipeline: wgpu::RenderPipeline,
  pub bg_layouts: Vec<RawBindGroupLayout>,
}

impl Deref for GPURenderPipeline {
  type Target = GPURenderPipelineInner;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
