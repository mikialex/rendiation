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
  Texture(GPUTextureView),
  ReusedTexture(Arc<Attachment>),
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
      RenderTargetView::ReusedTexture(t) => t.item().get_binding_build_source(),
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
      RenderTargetView::ReusedTexture(t) => t.item().get_bindable(),
    }
  }
}

impl From<GPU2DTextureView> for RenderTargetView {
  fn from(view: GPU2DTextureView) -> Self {
    Self::Texture(view.texture)
  }
}
impl From<Attachment> for RenderTargetView {
  fn from(view: Attachment) -> Self {
    Self::ReusedTexture(Arc::new(view))
  }
}

impl RenderTargetView {
  pub fn as_view(&self) -> &gpu::TextureView {
    match self {
      RenderTargetView::Texture(t) => t,
      RenderTargetView::SurfaceTexture { view, .. } => view.as_ref(),
      RenderTargetView::ReusedTexture(t) => t.item(),
    }
  }

  pub fn expect_standalone_common_texture_view(&self) -> &GPUTextureView {
    match self {
      RenderTargetView::Texture(t) => t,
      RenderTargetView::ReusedTexture(t) => t.item(),
      _ => panic!("expect_standalone_texture_view failed"),
    }
  }

  pub fn create_attachment_key(&self) -> PooledTextureKey {
    PooledTextureKey {
      size: self.size(),
      format: self.format(),
      sample_count: self.sample_count(),
    }
  }

  pub fn size(&self) -> Size {
    match self {
      RenderTargetView::Texture(t) => t.size_assume_2d(),
      RenderTargetView::SurfaceTexture { size, .. } => *size,
      RenderTargetView::ReusedTexture(t) => t.item().size_assume_2d(),
    }
  }

  pub fn format(&self) -> wgpu::TextureFormat {
    match self {
      RenderTargetView::Texture(t) => t.resource.desc.format,
      RenderTargetView::SurfaceTexture { format, .. } => *format,
      RenderTargetView::ReusedTexture(t) => t.item().resource.desc.format,
    }
  }

  pub fn sample_count(&self) -> u32 {
    match self {
      RenderTargetView::Texture(t) => t.resource.desc.sample_count,
      RenderTargetView::SurfaceTexture { .. } => 1,
      RenderTargetView::ReusedTexture(t) => t.item().resource.desc.sample_count,
    }
  }
}

/// Stored extra binding states info for up level usage
pub struct GPURenderPassCtx {
  pub pass: GPURenderPass,
  pub gpu: GPU,
  pub binding: BindingBuilder,
  incremental_vertex_binding_index: u32,
  pub enable_bind_check: bool,
}

impl GPURenderPassCtx {
  pub fn new(pass: GPURenderPass, gpu: GPU) -> Self {
    Self {
      pass,
      enable_bind_check: gpu.device.get_binding_ty_check_enabled(),
      gpu,
      binding: Default::default(),
      incremental_vertex_binding_index: 0,
    }
  }

  pub fn reset_vertex_binding_index(&mut self) {
    self.incremental_vertex_binding_index = 0;
  }

  pub fn set_vertex_buffer_by_buffer_resource_view_next(&mut self, buffer: &GPUBufferResourceView) {
    self
      .pass
      .set_vertex_buffer_by_buffer_resource_view(self.incremental_vertex_binding_index, buffer);
    self.incremental_vertex_binding_index += 1;
  }
}

#[derive(Clone, Hash)]
pub struct RenderTargetFormatsInfo {
  pub color_formats: Vec<wgpu::TextureFormat>,
  pub depth_stencil_formats: Option<wgpu::TextureFormat>,
  pub sample_count: u32,
}

pub struct GPURenderPass {
  pub pass: gpu::RenderPass<'static>,
  pub(crate) placeholder_bg: Arc<gpu::BindGroup>,
  pub(crate) size: Size,
  pub(crate) formats: RenderTargetFormatsInfo,
}

impl AbstractPassBinding for GPURenderPass {
  fn set_bind_group_placeholder(&mut self, index: u32) {
    self
      .pass
      .set_bind_group(index, self.placeholder_bg.as_ref(), &[]);
  }

  fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup, offsets: &[DynamicOffset]) {
    self.pass.set_bind_group(index, bind_group, offsets);
  }
}

impl Deref for GPURenderPass {
  type Target = gpu::RenderPass<'static>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl DerefMut for GPURenderPass {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

impl GPURenderPass {
  pub fn size(&self) -> Size {
    self.size
  }

  pub fn formats(&self) -> &RenderTargetFormatsInfo {
    &self.formats
  }

  pub fn set_gpu_pipeline(&mut self, pipeline: &GPURenderPipeline) {
    self.pass.set_pipeline(&pipeline.inner.as_ref().pipeline)
  }

  pub fn set_vertex_buffer_by_buffer_resource_view(
    &mut self,
    slot: u32,
    view: &GPUBufferResourceView,
  ) {
    let buffer = &view.resource.gpu;
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

  pub fn set_index_buffer_by_buffer_resource_view(
    &mut self,
    view: &GPUBufferResourceView,
    index_format: gpu::IndexFormat,
  ) {
    let buffer = &view.resource.gpu;
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
      DrawCommand::Indirect {
        indirect_buffer,
        indexed,
      } => {
        let buffer = &indirect_buffer.resource.gpu;
        if indexed {
          self.draw_indexed_indirect(buffer, 0)
        } else {
          self.draw_indirect(buffer, 0)
        }
      }
      DrawCommand::MultiIndirect {
        indirect_buffer,
        indexed,
        count,
      } => {
        let buffer = &indirect_buffer.resource.gpu;
        if indexed {
          self.multi_draw_indexed_indirect(buffer, 0, count)
        } else {
          self.multi_draw_indirect(buffer, 0, count)
        }
      }
      DrawCommand::MultiIndirectCount {
        indirect_buffer,
        indirect_count,
        indexed,
        max_count,
      } => {
        let buffer = &indirect_buffer.resource.gpu;
        let count_buffer = &indirect_count.resource.gpu;
        if indexed {
          self.multi_draw_indexed_indirect_count(buffer, 0, count_buffer, 0, max_count)
        } else {
          self.multi_draw_indirect_count(buffer, 0, count_buffer, 0, max_count)
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
  Indirect {
    indirect_buffer: GPUBufferResourceView,
    indexed: bool,
  },
  MultiIndirect {
    indirect_buffer: GPUBufferResourceView,
    indexed: bool,
    count: u32,
  },
  MultiIndirectCount {
    indexed: bool,
    indirect_buffer: GPUBufferResourceView,
    indirect_count: GPUBufferResourceView,
    max_count: u32,
  },
  Skip,
}

pub struct GPUComputePass {
  pub(crate) pass: gpu::ComputePass<'static>,
  pub(crate) placeholder_bg: Arc<gpu::BindGroup>,
}

impl AbstractPassBinding for GPUComputePass {
  fn set_bind_group_placeholder(&mut self, index: u32) {
    self
      .pass
      .set_bind_group(index, self.placeholder_bg.as_ref(), &[]);
  }

  fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup, offsets: &[DynamicOffset]) {
    self.pass.set_bind_group(index, bind_group, offsets);
  }
}

impl Deref for GPUComputePass {
  type Target = gpu::ComputePass<'static>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl DerefMut for GPUComputePass {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

impl GPUComputePass {
  pub fn set_gpu_pipeline(&mut self, pipeline: &GPUComputePipeline) {
    self.pass.set_pipeline(&pipeline.inner.as_ref().pipeline)
  }

  pub fn dispatch_workgroups_indirect_by_buffer_resource_view(
    &mut self,
    indirect_buffer: &GPUBufferResourceView,
  ) {
    self.dispatch_workgroups_indirect(&indirect_buffer.resource.gpu, 0)
  }
}
