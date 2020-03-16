pub trait TextureDimension {
  const WGPU_CONST: wgpu::TextureDimension;
  fn to_wgpu(&self) -> wgpu::Extent3d;
}

pub struct TextureSize2D {
  pub width: u32,
  pub height: u32,
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

pub struct TextureSize3D {
  pub width: u32,
  pub height: u32,
  pub depth: u32,
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
