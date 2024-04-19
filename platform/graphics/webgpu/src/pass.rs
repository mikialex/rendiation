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
    view: Arc<gpu::TextureView>,
    view_id: usize,
    bindgroup_holder: BindGroupResourceHolder,
  },
}

impl CacheAbleBindingSource for RenderTargetView {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    match self {
      RenderTargetView::Texture(t) => t.get_binding_build_source(),
      RenderTargetView::SurfaceTexture { view_id, .. } => CacheAbleBindingBuildSource {
        source: self.get_bindable(),
        view_id: *view_id,
      },
    }
  }
}

impl BindableResourceProvider for RenderTargetView {
  fn get_bindable(&self) -> BindingResourceOwned {
    match self {
      RenderTargetView::Texture(t) => t.get_bindable(),
      RenderTargetView::SurfaceTexture {
        view,
        bindgroup_holder,
        ..
      } => BindingResourceOwned::RawTextureView(view.clone(), bindgroup_holder.clone()),
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
          .mip_level_size(t.desc.base_mip_level, gpu::TextureDimension::D2);
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
pub struct GPURenderPassCtx<'encoder, 'gpu> {
  pub pass: GPURenderPass<'encoder>,
  pub gpu: &'gpu GPU,
  pub binding: BindingBuilder,
  incremental_vertex_binding_index: u32,
}

impl<'encoder, 'gpu> GPURenderPassCtx<'encoder, 'gpu> {
  pub fn new(pass: GPURenderPass<'encoder>, gpu: &'gpu GPU) -> Self {
    Self {
      pass,
      gpu,
      binding: Default::default(),
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

impl RenderPassDescriptorOwned {
  pub fn buffer_size(&self) -> Vec2<f32> {
    self
      .channels
      .first()
      .map(|c| &c.1)
      .or_else(|| self.depth_stencil_target.as_ref().map(|c| &c.1))
      .map(|c| Vec2::from(c.size().into_usize()).map(|v| v as f32))
      .unwrap_or_else(Vec2::zero)
  }
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
  pub(crate) placeholder_bg: Arc<gpu::BindGroup>,
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
  buffers: Arena<Arc<gpu::Buffer>>,
  bindgroups: Arena<Arc<gpu::BindGroup>>,
  graphics_pipelines: Arena<GPURenderPipeline>,
  compute_pipelines: Arena<GPUComputePipeline>,
}

impl<'a> GPURenderPass<'a> {
  pub fn size(&self) -> Size {
    self.size
  }

  pub fn formats(&self) -> &RenderTargetFormatsInfo {
    &self.formats
  }

  pub fn set_pipeline_owned(&mut self, pipeline: &GPURenderPipeline) {
    let pipeline = self.holder.graphics_pipelines.alloc(pipeline.clone());
    self.pass.set_pipeline(&pipeline.inner.as_ref().pipeline)
  }

  pub fn set_bind_group_placeholder(&mut self, index: u32) {
    self.set_bind_group_owned(index, &self.placeholder_bg.clone(), &[]);
  }

  pub fn set_bind_group_owned(
    &mut self,
    index: u32,
    bind_group: &Arc<gpu::BindGroup>,
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
      DrawCommand::Skip => {}
      DrawCommand::MultiIndirect {
        indirect_buffer,
        indexed,
        indirect_offset,
        count,
      } => {
        let buffer = self
          .holder
          .buffers
          .alloc(indirect_buffer.resource.gpu.clone());
        if indexed {
          self.multi_draw_indexed_indirect(buffer, indirect_offset, count)
        } else {
          self.multi_draw_indirect(buffer, indirect_offset, count)
        }
      }
    }
  }
}

#[derive(Clone)]
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
  MultiIndirect {
    indexed: bool,
    indirect_buffer: GPUBufferResourceView,
    indirect_offset: BufferAddress,
    count: u32,
  },
  Skip,
}

pub struct GPUComputePass<'a> {
  pub(crate) pass: gpu::ComputePass<'a>,
  pub(crate) holder: &'a GPURenderPassDataHolder,
  pub(crate) placeholder_bg: Arc<gpu::BindGroup>,
}

impl<'a> Deref for GPUComputePass<'a> {
  type Target = gpu::ComputePass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a> DerefMut for GPUComputePass<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

impl<'a> GPUComputePass<'a> {
  pub fn set_pipeline_owned(&mut self, pipeline: &GPUComputePipeline) {
    let pipeline = self.holder.compute_pipelines.alloc(pipeline.clone());
    self.pass.set_pipeline(&pipeline.inner.as_ref().pipeline)
  }

  pub fn set_bind_group_placeholder(&mut self, index: u32) {
    self.set_bind_group_owned(index, &self.placeholder_bg.clone(), &[]);
  }

  pub fn set_bind_group_owned(
    &mut self,
    index: u32,
    bind_group: &Arc<gpu::BindGroup>,
    offsets: &[gpu::DynamicOffset],
  ) {
    let bind_group = self.holder.bindgroups.alloc(bind_group.clone());
    self.set_bind_group(index, bind_group, offsets)
  }
}
