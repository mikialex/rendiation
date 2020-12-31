pub trait TextureDimension: Copy + Clone {
  const WGPU_CONST: wgpu::TextureDimension;
  fn to_wgpu(&self) -> wgpu::Extent3d;
  fn get_pixel_size(&self) -> u32 {
    let gpu = self.to_wgpu();
    gpu.width * gpu.height * gpu.depth
  }
}

#[derive(Debug, Copy, Clone)]
pub struct TextureSize2D {
  pub width: u32,
  pub height: u32,
}

impl TextureSize2D {
  pub fn to_tuple(&self) -> (usize, usize) {
    (self.width as usize, self.height as usize)
  }
}

impl From<(usize, usize)> for TextureSize2D {
  fn from(size: (usize, usize)) -> Self {
    Self {
      width: size.0 as u32,
      height: size.1 as u32,
    }
  }
}

impl TextureDimension for TextureSize2D {
  const WGPU_CONST: wgpu::TextureDimension = wgpu::TextureDimension::D2;
  fn to_wgpu(&self) -> wgpu::Extent3d {
    wgpu::Extent3d {
      width: self.width,
      height: self.height,
      depth: 1,
    }
  }
}

#[derive(Debug, Copy, Clone)]
pub struct TextureSize3D {
  pub width: u32,
  pub height: u32,
  pub depth: u32,
}

impl From<(usize, usize, usize)> for TextureSize3D {
  fn from(size: (usize, usize, usize)) -> Self {
    Self {
      width: size.0 as u32,
      height: size.1 as u32,
      depth: size.2 as u32,
    }
  }
}

impl TextureDimension for TextureSize3D {
  const WGPU_CONST: wgpu::TextureDimension = wgpu::TextureDimension::D2;
  fn to_wgpu(&self) -> wgpu::Extent3d {
    wgpu::Extent3d {
      width: self.width,
      height: self.height,
      depth: self.depth,
    }
  }
}
