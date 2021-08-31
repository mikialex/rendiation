use rendiation_texture_types::Size;

use crate::GPU;

pub trait GPUTextureSize {
  fn into_gpu_size(&self) -> wgpu::Extent3d;
}

impl GPUTextureSize for Size {
  fn into_gpu_size(&self) -> wgpu::Extent3d {
    wgpu::Extent3d {
      width: usize::from(self.width) as u32,
      height: usize::from(self.height) as u32,
      depth_or_array_layers: 1,
    }
  }
}

pub struct RenderTarget {
  attachments: Vec<(wgpu::Texture, wgpu::TextureFormat)>,
  depth: Option<(wgpu::Texture, wgpu::TextureFormat)>,
  size: Size,
}

impl RenderTarget {
  pub fn resize(&mut self, gpu: &GPU, size: Size) {
    self.size = size;
    self
      .attachments
      .iter_mut()
      .for_each(|(t, f)| *t = build_attachment(size, *f, gpu));
    self
      .depth
      .as_mut()
      .map(|(t, f)| *t = build_attachment(size, *f, gpu));
  }
}

pub struct RenderTargetBuilder<'a> {
  gpu: &'a GPU,
  target: RenderTarget,
}

fn build_attachment(size: Size, format: wgpu::TextureFormat, gpu: &GPU) -> wgpu::Texture {
  gpu.device.create_texture(&wgpu::TextureDescriptor {
    size: size.into_gpu_size(),
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format,
    usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
    label: None,
  })
}

impl<'a> RenderTargetBuilder<'a> {
  pub fn new(gpu: &'a GPU, size: Size) -> Self {
    Self {
      gpu,
      target: RenderTarget {
        attachments: Vec::new(),
        depth: None,
        size,
      },
    }
  }

  pub fn with_color(mut self, format: wgpu::TextureFormat) -> Self {
    let attachment = build_attachment(self.target.size, format, self.gpu);
    self.target.attachments.push((attachment, format));
    self
  }

  pub fn with_depth(mut self, format: DepthStencilTextureFormat) -> Self {
    let format = format.into();
    let attachment = build_attachment(self.target.size, format, self.gpu);
    self.target.depth = (attachment, format).into();
    self
  }

  pub fn build(self) -> RenderTarget {
    self.target
  }
}

#[derive(Copy, Clone)]
pub enum DepthStencilTextureFormat {
  /// Special depth format with 32 bit floating point depth.
  Depth32Float = 35,
  /// Special depth format with at least 24 bit integer depth.
  Depth24Plus = 36,
  /// Special depth/stencil format with at least 24 bit integer depth and 8 bits integer stencil.
  Depth24PlusStencil8 = 37,
}

impl From<DepthStencilTextureFormat> for wgpu::TextureFormat {
  fn from(value: DepthStencilTextureFormat) -> Self {
    match value {
      DepthStencilTextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
      DepthStencilTextureFormat::Depth24Plus => wgpu::TextureFormat::Depth24Plus,
      DepthStencilTextureFormat::Depth24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
    }
  }
}
