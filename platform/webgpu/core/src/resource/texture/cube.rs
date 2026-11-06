use crate::*;

/// The upload methods without try prefix panic if the source format is not copyable.
impl GPUCubeTexture {
  #[must_use]
  pub fn upload(
    self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    face: CubeTextureFace,
    mip_level: usize,
  ) -> Self {
    expect_copyable(self.try_upload(queue, source, face, mip_level))
  }

  pub fn try_upload(
    self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    face: CubeTextureFace,
    mip_level: usize,
  ) -> Result<Self, TextureFormatNotCopyable> {
    self.try_upload_with_origin(queue, source, face, mip_level, (0, 0))
  }

  #[must_use]
  pub fn upload_with_origin(
    self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    face: CubeTextureFace,
    mip_level: usize,
    origin: (usize, usize),
  ) -> Self {
    expect_copyable(self.try_upload_with_origin(queue, source, face, mip_level, origin))
  }

  pub fn try_upload_with_origin(
    self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    face: CubeTextureFace,
    mip_level: usize,
    origin: (usize, usize),
  ) -> Result<Self, TextureFormatNotCopyable> {
    let bytes_per_row = source.bytes_per_row()?;
    queue.write_texture(
      gpu::TexelCopyTextureInfo {
        texture: self.texture.gpu_resource().gpu(),
        mip_level: mip_level as u32,
        origin: gpu::Origin3d {
          x: origin.0 as u32,
          y: origin.1 as u32,
          z: face as u32,
        },
        aspect: gpu::TextureAspect::All,
      },
      source.as_bytes(),
      gpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    Ok(self)
  }
}
