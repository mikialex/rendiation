use crate::*;

/// The wrapper type that make sure the inner desc
/// is suitable for cube texture
pub struct WebGPUTextureCubeDescriptor {
  pub(crate) desc: gpu::TextureDescriptor<'static>,
}

pub struct GPURawTextureCube(pub gpu::Texture);
pub struct GPURawTextureCubeView(pub gpu::TextureView);

pub type GPUTextureCube = ResourceRc<GPURawTextureCube>;
pub type GPUTextureCubeView = ResourceViewRc<GPURawTextureCube>;

impl Resource for GPURawTextureCube {
  type Descriptor = WebGPUTextureCubeDescriptor;

  type View = GPURawTextureCubeView;

  type ViewDescriptor = ();

  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    let desc = &desc.desc;
    GPURawTextureCube(device.create_texture(desc))
  }

  fn create_view(&self, _desc: &Self::ViewDescriptor) -> Self::View {
    GPURawTextureCubeView(self.0.create_view(&gpu::TextureViewDescriptor {
      dimension: Some(gpu::TextureViewDimension::Cube),
      ..gpu::TextureViewDescriptor::default()
    }))
  }
}

impl GPUTextureCube {
  #[must_use]
  pub fn upload(
    self,
    queue: &gpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    face: CubeTextureFace,
    mip_level: usize,
  ) -> Self {
    self.upload_with_origin(queue, source, face, mip_level, (0, 0))
  }

  #[must_use]
  pub fn upload_with_origin(
    self,
    queue: &gpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    face: CubeTextureFace,
    mip_level: usize,
    origin: (usize, usize),
  ) -> Self {
    // validation
    queue.write_texture(
      gpu::ImageCopyTexture {
        texture: &self.0,
        mip_level: mip_level as u32,
        origin: gpu::Origin3d {
          x: origin.0 as u32,
          y: origin.1 as u32,
          z: face as u32,
        },
        aspect: gpu::TextureAspect::All,
      },
      source.as_bytes(),
      gpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    self
  }
}
