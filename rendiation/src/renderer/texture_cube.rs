pub struct WGPUTextureCube {
  texture: WGPUTexture,
}

impl WGPUTextureCube {
  pub fn new(
    size: (usize, usize),
    px: &[u8],
    nx: &[u8],
    py: &[u8],
    ny: &[u8],
    pz: &[u8],
    nz: &[u8],
  ) -> Self {
      
    let size: TextureSize2D = size.into();
    let descriptor = wgpu::TextureDescriptor {
      size: size.to_wgpu(),
      array_layer_count: 1,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    
  }

  pub fn upload_all(
    size: (usize, usize),
    px: &[u8],
    nx: &[u8],
    py: &[u8],
    ny: &[u8],
    pz: &[u8],
    nz: &[u8],
  ){

  }
}
