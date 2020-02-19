use crate::renderer::buffer::WGPUBuffer;

pub struct WGPUTexture {
  gpu_texture: wgpu::Texture,
  descriptor: wgpu::TextureDescriptor,
  view: wgpu::TextureView,
}

impl WGPUTexture {
  pub fn new_as_depth(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (usize, usize),
  ) -> Self {
    let descriptor = wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: size.0 as u32,
        height: size.1 as u32,
        depth: 1,
      },
      array_layer_count: 1,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    let depth_texture = device.create_texture(&descriptor);
    let view = depth_texture.create_default_view();
    Self {
      descriptor,
      gpu_texture: depth_texture,
      view,
    }
  }

  pub fn new_as_target(device: &wgpu::Device, size: (u32, u32)) -> Self {
    let (width, height) = size;
    let descriptor = wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width,
        height,
        depth: 1,
      },
      array_layer_count: 1,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    let gpu_texture = device.create_texture(&descriptor);
    let view = gpu_texture.create_default_view();
    Self {
      gpu_texture,
      descriptor,
      view,
    }
  }

  pub fn new_from_image_data(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    data: &[u8],
    size: (u32, u32, u32),
  ) -> WGPUTexture {
    let (width, height, depth) = size;
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
    let view = gpu_texture.create_default_view();
    let wgpu_texture = Self {
      gpu_texture,
      descriptor,
      view,
    };

    wgpu_texture.upload(device, encoder, data);
    wgpu_texture
  }

  pub fn view(&self) -> &wgpu::TextureView {
    &self.view
  }

  pub fn format(&self) -> &wgpu::TextureFormat {
    &self.descriptor.format
  }

  /// this will not keep content resize, just recreate the gpu resource with new size
  pub fn resize(&mut self, device: &wgpu::Device, size: (usize, usize)) {
    self.descriptor.size.width = size.0 as u32;
    self.descriptor.size.height = size.1 as u32;
    self.gpu_texture = device.create_texture(&self.descriptor);
    self.view = self.gpu_texture.create_default_view();
  }

  fn upload(&self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, image_data: &[u8]) {
    let buffer = WGPUBuffer::new(device, image_data, wgpu::BufferUsage::COPY_SRC);

    encoder.copy_buffer_to_texture(
      wgpu::BufferCopyView {
        buffer: buffer.get_gpu_buffer(),
        offset: 0,
        row_pitch: 4 * self.descriptor.size.width,
        image_height: self.descriptor.size.height,
      },
      wgpu::TextureCopyView {
        texture: &self.gpu_texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d {
          x: 0.0,
          y: 0.0,
          z: 0.0,
        },
      },
      self.descriptor.size,
    );
  }
}
