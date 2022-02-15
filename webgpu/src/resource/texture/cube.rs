use rendiation_texture_types::CubeTextureFace;

use crate::*;

/// The wrapper type that make sure the inner desc
/// is suitable for cube texture
pub struct WebGPUTextureCubeDescriptor {
  pub(crate) desc: wgpu::TextureDescriptor<'static>,
}

pub struct GPURawTextureCube(pub(crate) wgpu::Texture);
pub struct GPURawTextureCubeView(pub(crate) wgpu::TextureView);

pub type GPUTextureCube = ResourceRc<GPURawTextureCube>;

impl Resource for GPURawTextureCube {
  type Descriptor = WebGPUTextureCubeDescriptor;

  type View = GPURawTextureCubeView;

  type ViewDescriptor = ();

  fn create_resource(desc: &Self::Descriptor, device: &wgpu::Device) -> Self {
    let desc = &desc.desc;
    GPURawTextureCube(device.create_texture(desc))
  }

  fn create_view(&self, desc: &Self::ViewDescriptor, device: &wgpu::Device) -> Self::View {
    GPURawTextureCubeView(self.0.create_view(&wgpu::TextureViewDescriptor {
      dimension: Some(wgpu::TextureViewDimension::Cube),
      ..wgpu::TextureViewDescriptor::default()
    }))
  }
}

impl GPUTextureCube {
  #[must_use]
  pub fn upload(
    self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    face: CubeTextureFace,
    mip_level: usize,
  ) -> Self {
    self.upload_with_origin(queue, source, face, mip_level, (0, 0))
  }

  #[must_use]
  pub fn upload_with_origin(
    self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    face: CubeTextureFace,
    mip_level: usize,
    origin: (usize, usize),
  ) -> Self {
    // validation
    queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &self.0,
        mip_level: mip_level as u32,
        origin: wgpu::Origin3d {
          x: origin.0 as u32,
          y: origin.1 as u32,
          z: face as u32,
        },
        aspect: wgpu::TextureAspect::All,
      },
      source.as_bytes(),
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    self
  }
}
