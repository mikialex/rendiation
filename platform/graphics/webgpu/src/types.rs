use std::num::NonZeroU32;

use crate::*;

pub trait GPUTextureSize {
  fn into_gpu_size(self) -> gpu::Extent3d;
  fn from_gpu_size(size: gpu::Extent3d) -> Self;
}

impl GPUTextureSize for Size {
  fn into_gpu_size(self) -> gpu::Extent3d {
    gpu::Extent3d {
      width: usize::from(self.width) as u32,
      height: usize::from(self.height) as u32,
      depth_or_array_layers: 1,
    }
  }
  fn from_gpu_size(size: gpu::Extent3d) -> Self {
    Size {
      width: NonZeroUsize::new(size.width as usize).unwrap(),
      height: NonZeroUsize::new(size.height as usize).unwrap(),
    }
  }
}

impl GPUTextureSize for SizeWithDepth {
  fn into_gpu_size(self) -> gpu::Extent3d {
    gpu::Extent3d {
      width: usize::from(self.size.width) as u32,
      height: usize::from(self.size.height) as u32,
      depth_or_array_layers: self.depth.into(),
    }
  }
  fn from_gpu_size(size: gpu::Extent3d) -> Self {
    SizeWithDepth {
      size: Size {
        width: NonZeroUsize::new(size.width as usize).unwrap(),
        height: NonZeroUsize::new(size.height as usize).unwrap(),
      },
      depth: NonZeroU32::new(size.depth_or_array_layers).unwrap(),
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

impl From<DepthStencilTextureFormat> for gpu::TextureFormat {
  fn from(value: DepthStencilTextureFormat) -> Self {
    match value {
      DepthStencilTextureFormat::Depth32Float => gpu::TextureFormat::Depth32Float,
      DepthStencilTextureFormat::Depth24Plus => gpu::TextureFormat::Depth24Plus,
      DepthStencilTextureFormat::Depth24PlusStencil8 => gpu::TextureFormat::Depth24PlusStencil8,
    }
  }
}
