use crate::renderer::buffer::WGPUBuffer;

pub trait Image{
  fn get_size(&self) -> (u32, u32, u32);
  fn get_data(&self) -> &[u8];
}

pub struct WGPUTexture {
  gpu_texture: wgpu::Texture,
  descriptor: wgpu::TextureDescriptor,

  buffer: WGPUBuffer,
}

impl WGPUTexture{
  pub fn new<Img: Image>(device: &wgpu::Device, encoder: &wgpu::CommandEncoder, value: Img) -> WGPUTexture{
    let (width, height, depth) = value.get_size();
    let descriptor = wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width,
        height,
        depth,
      },
      array_layer_count: 1,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    };
    let gpu_texture = device.create_texture(&descriptor);

    let buffer = WGPUBuffer::new(device, value.get_data(), wgpu::BufferUsage::COPY_SRC);

    WGPUTexture{
      gpu_texture,
      descriptor,
      buffer,
    }

  }

  pub fn update(device: &wgpu::Device, encoder: &wgpu::CommandEncoder, value: WGPUBuffer){
    self.value = value;

    init_encoder.copy_buffer_to_texture(
      wgpu::BufferCopyView {
        buffer: &temp_buf,
        offset: 0,
        row_pitch: 4 * size,
        image_height: size,
      },
      wgpu::TextureCopyView {
        texture: &texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d {
          x: 0.0,
          y: 0.0,
          z: 0.0,
        },
      },
      texture_extent,
    );
  }
}

// fn update_texture