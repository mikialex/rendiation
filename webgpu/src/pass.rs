use crate::*;

pub trait ShaderPassBuilder {
  fn setup_pass(&self, _ctx: &mut GPURenderPassCtx) {}
  fn post_setup_pass(&self, _ctx: &mut GPURenderPassCtx) {}

  fn setup_pass_self(&self, ctx: &mut GPURenderPassCtx) {
    self.setup_pass(ctx);
    self.post_setup_pass(ctx);
  }
}

impl ShaderPassBuilder for () {}

#[derive(Clone)]
pub enum RenderTargetView {
  Texture(GPU2DTextureView),
  SurfaceTexture {
    size: Size,
    format: gpu::TextureFormat,
    view: Rc<gpu::TextureView>,
    view_id: usize,
    /// when resource dropped, all referenced bindgroup should drop
    invalidation_tokens: Rc<RefCell<Vec<BindGroupCacheInvalidation>>>,
  },
}

impl BindableResourceView for RenderTargetView {
  fn as_bindable(&self) -> gpu::BindingResource {
    match self {
      RenderTargetView::Texture(t) => t.as_bindable(),
      RenderTargetView::SurfaceTexture { view, .. } => gpu::BindingResource::TextureView(view),
    }
  }
}

impl BindProvider for RenderTargetView {
  fn view_id(&self) -> usize {
    match self {
      RenderTargetView::Texture(t) => t.view_id(),
      RenderTargetView::SurfaceTexture { view_id, .. } => *view_id,
    }
  }

  fn add_bind_record(&self, record: BindGroupCacheInvalidation) {
    match self {
      RenderTargetView::Texture(t) => t.add_bind_record(record),
      RenderTargetView::SurfaceTexture {
        invalidation_tokens,
        ..
      } => invalidation_tokens.borrow_mut().push(record),
    }
  }
}

impl From<GPU2DTextureView> for RenderTargetView {
  fn from(view: GPU2DTextureView) -> Self {
    Self::Texture(view)
  }
}

impl RenderTargetView {
  pub fn as_view(&self) -> &gpu::TextureView {
    match self {
      RenderTargetView::Texture(t) => &t.view,
      RenderTargetView::SurfaceTexture { view, .. } => view.as_ref(),
    }
  }

  pub fn size(&self) -> Size {
    match self {
      RenderTargetView::Texture(t) => {
        let size = t
          .resource
          .desc
          .size
          .mip_level_size(t.desc.base_mip_level, false);
        GPUTextureSize::from_gpu_size(size)
      }
      RenderTargetView::SurfaceTexture { size, .. } => *size,
    }
  }

  pub fn format(&self) -> wgpu::TextureFormat {
    match self {
      RenderTargetView::Texture(t) => t.resource.desc.format,
      RenderTargetView::SurfaceTexture { format, .. } => *format,
    }
  }

  pub fn sample_count(&self) -> u32 {
    match self {
      RenderTargetView::Texture(t) => t.resource.desc.sample_count,
      RenderTargetView::SurfaceTexture { .. } => 1,
    }
  }
}

/// Stored extra binding states info for up level usage
pub struct GPURenderPassCtx<'a, 'b> {
  pub pass: GPURenderPass<'a>,
  pub gpu: &'b GPU,
  pub binding: BindingBuilder,
  incremental_vertex_binding_index: u32,
}

impl<'a, 'b> GPURenderPassCtx<'a, 'b> {
  pub fn new(pass: GPURenderPass<'a>, gpu: &'b GPU) -> Self {
    Self {
      pass,
      gpu,
      binding: gpu.device.create_binding_builder(),
      incremental_vertex_binding_index: 0,
    }
  }

  pub fn reset_vertex_binding_index(&mut self) {
    self.incremental_vertex_binding_index = 0;
  }

  pub fn set_vertex_buffer_owned_next(&mut self, buffer: &GPUBufferResourceView) {
    self
      .pass
      .set_vertex_buffer_owned(self.incremental_vertex_binding_index, buffer);
    self.incremental_vertex_binding_index += 1;
  }
}

#[derive(Default, Clone)]
pub struct RenderPassDescriptorOwned {
  pub name: String,
  pub channels: Vec<(gpu::Operations<gpu::Color>, RenderTargetView)>,
  pub depth_stencil_target: Option<(gpu::Operations<f32>, RenderTargetView)>,
  pub resolve_target: Option<RenderTargetView>,
}

#[derive(Clone, Hash)]
pub struct RenderTargetFormatsInfo {
  pub color_formats: Vec<wgpu::TextureFormat>,
  pub depth_stencil_formats: Option<wgpu::TextureFormat>,
  pub sample_count: u32,
}

pub struct GPURenderPass<'a> {
  pub(crate) pass: gpu::RenderPass<'a>,
  pub(crate) holder: &'a GPURenderPassDataHolder,
  pub(crate) placeholder_bg: Rc<gpu::BindGroup>,
  pub(crate) size: Size,
  pub(crate) formats: RenderTargetFormatsInfo,
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
  pipelines: Arena<GPURenderPipeline>,
}

impl<'a> GPURenderPass<'a> {
  pub fn size(&self) -> Size {
    self.size
  }

  pub fn formats(&self) -> &RenderTargetFormatsInfo {
    &self.formats
  }

  pub fn set_pipeline_owned(&mut self, pipeline: &GPURenderPipeline) {
    let pipeline = self.holder.pipelines.alloc(pipeline.clone());
    self.pass.set_pipeline(&pipeline.inner.as_ref().pipeline)
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

  pub fn set_vertex_buffer_owned(&mut self, slot: u32, view: &GPUBufferResourceView) {
    let buffer = self
      .holder
      .buffers
      .alloc(view.resource.resource.gpu.clone());

    // why this so stupid
    if let Some(size) = view.desc.size {
      self.pass.set_vertex_buffer(
        slot,
        buffer.slice(view.desc.offset..(view.desc.offset + u64::from(size))),
      )
    } else {
      self
        .pass
        .set_vertex_buffer(slot, buffer.slice(view.desc.offset..))
    }
  }

  pub fn set_index_buffer_owned(
    &mut self,
    view: &GPUBufferResourceView,
    index_format: gpu::IndexFormat,
  ) {
    let buffer = self
      .holder
      .buffers
      .alloc(view.resource.resource.gpu.clone());
    // why this so stupid
    if let Some(size) = view.desc.size {
      self.pass.set_index_buffer(
        buffer.slice(view.desc.offset..(view.desc.offset + u64::from(size))),
        index_format,
      )
    } else {
      self
        .pass
        .set_index_buffer(buffer.slice(view.desc.offset..), index_format)
    }
  }

  pub fn draw_by_command(&mut self, com: DrawCommand) {
    match com {
      DrawCommand::Indexed {
        base_vertex,
        indices,
        instances,
      } => self.draw_indexed(indices, base_vertex, instances),
      DrawCommand::Array {
        vertices,
        instances,
      } => self.draw(vertices, instances),
      _ => {}
    }
  }
}

pub enum DrawCommand {
  Indexed {
    base_vertex: i32,
    indices: Range<u32>,
    instances: Range<u32>,
  },
  Array {
    vertices: Range<u32>,
    instances: Range<u32>,
  },
  Skip,
}
