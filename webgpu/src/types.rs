use rendiation_texture_types::Size;

pub trait GPUTextureSize {
  fn into_gpu_size(self) -> wgpu::Extent3d;
}

impl GPUTextureSize for Size {
  fn into_gpu_size(self) -> wgpu::Extent3d {
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

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexBufferLayoutOwned {
  /// The stride, in bytes, between elements of this buffer.
  pub array_stride: wgpu::BufferAddress,
  /// How often this vertex buffer is "stepped" forward.
  pub step_mode: wgpu::VertexStepMode,
  /// The list of attributes which comprise a single vertex.
  pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexBufferLayoutOwned {
  pub fn as_raw(&self) -> wgpu::VertexBufferLayout {
    wgpu::VertexBufferLayout {
      array_stride: self.array_stride,
      step_mode: self.step_mode,
      attributes: self.attributes.as_slice(),
    }
  }
}

impl<'a> From<wgpu::VertexBufferLayout<'a>> for VertexBufferLayoutOwned {
  fn from(layout: wgpu::VertexBufferLayout<'a>) -> Self {
    Self {
      array_stride: layout.array_stride,
      step_mode: layout.step_mode,
      attributes: layout.attributes.to_owned(),
    }
  }
}
