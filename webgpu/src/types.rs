use crate::*;

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
