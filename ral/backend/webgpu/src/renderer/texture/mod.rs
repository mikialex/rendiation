use crate::renderer::WGPURenderer;

pub mod texture_cube;
pub mod texture_dimension;
pub mod texture_format;
use crate::renderer::texture_dimension::*;
use crate::renderer::texture_format::*;

pub struct WGPUTexture<V: TextureDimension = TextureSize2D> {
  gpu_texture: wgpu::Texture,
  descriptor: wgpu::TextureDescriptor<'static>,
  size: V,
  view: wgpu::TextureView,
  pub format: TextureFormat, // todo improvement
}

impl WGPUTexture {
  pub fn new_as_depth(
    renderer: &WGPURenderer,
    format: wgpu::TextureFormat,
    size: (usize, usize),
  ) -> Self {
    let size: TextureSize2D = size.into();
    let descriptor = wgpu::TextureDescriptor {
      label: None,
      size: size.to_wgpu(),
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureSize2D::WGPU_CONST,
      format,
      usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
    };
    let depth_texture = renderer.device.create_texture(&descriptor);
    let view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    Self {
      descriptor,
      gpu_texture: depth_texture,
      view,
      size,
      format: TextureFormat::Rgba8UnormSrgb,
    }
  }

  pub fn new_as_target_default(renderer: &WGPURenderer, size: (usize, usize)) -> Self {
    WGPUTexture::new_as_target(renderer, TextureFormat::Rgba8UnormSrgb, size)
  }

  pub fn new_as_target(
    renderer: &WGPURenderer,
    format: TextureFormat,
    size: (usize, usize),
  ) -> Self {
    let size: TextureSize2D = size.into();
    let descriptor = wgpu::TextureDescriptor {
      label: None,
      size: size.to_wgpu(),
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureSize2D::WGPU_CONST,
      format: format.get_wgpu_format(),
      usage: wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::RENDER_ATTACHMENT,
    };
    let gpu_texture = renderer.device.create_texture(&descriptor);
    let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
    Self {
      gpu_texture,
      descriptor,
      view,
      size,
      format,
    }
  }

  pub fn new_from_image_data(
    renderer: &mut WGPURenderer,
    data: &[u8],
    size: (u32, u32, u32),
  ) -> WGPUTexture {
    let (width, height, depth) = size;
    let descriptor = wgpu::TextureDescriptor {
      label: None,
      size: wgpu::Extent3d {
        width,
        height,
        depth,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureSize2D::WGPU_CONST,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    };
    let gpu_texture = renderer.device.create_texture(&descriptor);
    let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let wgpu_texture = Self {
      gpu_texture,
      descriptor,
      view,
      size: TextureSize2D {
        width: size.0 as u32,
        height: size.1 as u32,
      },
      format: TextureFormat::Rgba8UnormSrgb,
    };

    wgpu_texture.upload(renderer, data);
    wgpu_texture
  }

  pub fn view(&self) -> &wgpu::TextureView {
    &self.view
  }

  pub fn size(&self) -> TextureSize2D {
    self.size
  }

  pub fn format(&self) -> &wgpu::TextureFormat {
    &self.descriptor.format
  }

  /// this will not keep content resize, just recreate the gpu resource with new size
  pub fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize)) {
    self.descriptor.size.width = size.0 as u32;
    self.descriptor.size.height = size.1 as u32;
    self.gpu_texture = renderer.device.create_texture(&self.descriptor);
    self.view = self
      .gpu_texture
      .create_view(&wgpu::TextureViewDescriptor::default());
  }

  fn upload(&self, renderer: &mut WGPURenderer, image_data: &[u8]) {
    upload(renderer, &self, image_data, 0)
  }
}

// impl<V: TextureDimension> WGPUTexture<V> {
//   pub async fn read(
//     &self,
//     renderer: &mut WGPURenderer,
//   ) -> Result<wgpu::BufferReadMapping, wgpu::BufferAsyncErr> {
//     let pixel_count = self.size.get_pixel_size() as u64;
//     let data_size = pixel_count * self.format.get_pixel_data_stride() as u64;

//     let output_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
//       label: None,
//       size: data_size,
//       usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
//     });

//     let buffer_future = output_buffer.map_read(0, data_size);

//     renderer.device.poll(wgpu::Maintain::Wait);

//     buffer_future.await
//   }
// }

pub fn upload(
  renderer: &mut WGPURenderer,
  texture: &WGPUTexture,
  image_data: &[u8],
  target_layer: u32,
) {
  renderer.queue.write_texture(
    wgpu::TextureCopyView {
      texture: &texture.gpu_texture,
      mip_level: 0,
      origin: wgpu::Origin3d {
        x: 0,
        y: 0,
        z: target_layer,
      },
    },
    image_data,
    wgpu::TextureDataLayout {
      offset: 0,
      bytes_per_row: 4 * texture.descriptor.size.width, // todo 4
      rows_per_image: 0,
    },
    texture.size.to_wgpu(),
  );
}
