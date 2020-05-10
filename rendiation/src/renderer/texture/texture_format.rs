use std::mem::size_of;

pub enum TextureFormat {
  Rgba8UnormSrgb,
}

impl TextureFormat {
  pub fn get_pixel_data_stride(&self) -> usize {
    match self {
      TextureFormat::Rgba8UnormSrgb => size_of::<u32>(),
    }
  }

  pub fn get_wgpu_format(&self) -> wgpu::TextureFormat {
    match self {
      TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
    }
  }
}
