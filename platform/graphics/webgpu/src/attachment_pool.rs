use crate::*;

pub type AttachmentPool = ReuseKVPool<PooledTextureKey, RenderViewContent>;
pub type Attachment = ReuseableItem<PooledTextureKey, RenderViewContent>;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct PooledTextureKey {
  pub size: Size,
  pub format: gpu::TextureFormat,
  pub sample_count: u32,
  pub require_mipmaps: bool,
  pub usage: TextureUsages,
}

impl PooledTextureKey {
  pub fn create_directly(self, gpu: &GPU) -> RenderViewContent {
    let mip_level_count = if self.require_mipmaps {
      MipLevelCount::BySize.get_level_count_wgpu(self.size)
    } else {
      1
    };
    let texture = GPUTexture::create(
      gpu::TextureDescriptor {
        label: None,
        size: map_size_gpu(self.size),
        dimension: gpu::TextureDimension::D2,
        format: self.format,
        view_formats: &[],
        usage: self.usage,
        mip_level_count,
        sample_count: self.sample_count,
      },
      &gpu.device,
    );
    let render_view = texture.create_view(gpu::TextureViewDescriptor {
      base_mip_level: 0,
      mip_level_count: Some(1),
      ..Default::default()
    });

    RenderViewContent::from_texture_view(render_view)
  }
}

pub fn init_attachment_pool(gpu: &GPU) -> AttachmentPool {
  let gpu = gpu.clone();
  ReuseKVPool::new(move |k: &PooledTextureKey| k.create_directly(&gpu))
}
