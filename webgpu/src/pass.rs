use crate::*;

#[derive(Clone, Hash)]
pub struct PassTargetFormatInfo {
  pub depth_stencil_format: Option<gpu::TextureFormat>,
  pub color_formats: Vec<gpu::TextureFormat>,
  pub sample_count: u32,
}

impl Default for PassTargetFormatInfo {
  fn default() -> Self {
    Self {
      depth_stencil_format: Default::default(),
      color_formats: Default::default(),
      sample_count: 1,
    }
  }
}

#[derive(Clone)]
pub enum ColorChannelView {
  Texture(GPUTexture2dView),
  SurfaceTexture(Rc<gpu::TextureView>),
}

impl From<GPUTexture2dView> for ColorChannelView {
  fn from(view: GPUTexture2dView) -> Self {
    Self::Texture(view)
  }
}

impl ColorChannelView {
  pub fn as_view(&self) -> &gpu::TextureView {
    match self {
      ColorChannelView::Texture(t) => &t.view.0,
      ColorChannelView::SurfaceTexture(v) => v.as_ref(),
    }
  }
}

#[derive(Default)]
pub struct RenderPassDescriptorOwned {
  pub name: String,
  pub channels: Vec<(gpu::Operations<gpu::Color>, ColorChannelView)>,
  pub depth_stencil_target: Option<(gpu::Operations<f32>, ColorChannelView)>,
  pub resolve_target: Option<ColorChannelView>,
}

pub struct GPURenderPass<'a> {
  pub(crate) pass: gpu::RenderPass<'a>,
  pub(crate) holder: &'a GPURenderPassDataHolder,
  pub(crate) placeholder_bg: Rc<gpu::BindGroup>,
}

impl<'a> Deref for GPURenderPass<'a> {
  type Target = gpu::RenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a> DerefMut for GPURenderPass<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

#[derive(Default)]
pub struct GPURenderPassDataHolder {
  buffers: Arena<Rc<gpu::Buffer>>,
  bindgroups: Arena<Rc<gpu::BindGroup>>,
  pipelines: Arena<Rc<gpu::RenderPipeline>>,
}

impl<'a> GPURenderPass<'a> {
  pub fn set_pipeline_owned(&mut self, pipeline: &Rc<gpu::RenderPipeline>) {
    let pipeline = self.holder.pipelines.alloc(pipeline.clone());
    self.pass.set_pipeline(pipeline)
  }

  pub fn set_bind_group_placeholder(&mut self, index: u32) {
    self.set_bind_group_owned(index, &self.placeholder_bg.clone(), &[]);
  }

  pub fn set_bind_group_owned(
    &mut self,
    index: u32,
    bind_group: &Rc<gpu::BindGroup>,
    offsets: &[gpu::DynamicOffset],
  ) {
    let bind_group = self.holder.bindgroups.alloc(bind_group.clone());
    self.set_bind_group(index, bind_group, offsets)
  }

  pub fn set_vertex_buffer_owned(&mut self, slot: u32, buffer: &Rc<gpu::Buffer>) {
    let buffer = self.holder.buffers.alloc(buffer.clone());
    self.pass.set_vertex_buffer(slot, buffer.slice(..))
  }

  pub fn set_index_buffer_owned(
    &mut self,
    buffer: &Rc<gpu::Buffer>,
    index_format: gpu::IndexFormat,
  ) {
    let buffer = self.holder.buffers.alloc(buffer.clone());
    self.pass.set_index_buffer(buffer.slice(..), index_format)
  }
}
