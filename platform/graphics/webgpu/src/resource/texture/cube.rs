use crate::*;

impl GPUCubeTexture {
  #[must_use]
  pub fn upload(
    self,
    queue: &gpu::Queue,
    source: &dyn WebGPU2DTextureSource,
    face: CubeTextureFace,
    mip_level: usize,
  ) -> Self {
    self.upload_with_origin(queue, source, face, mip_level, (0, 0))
  }

  #[must_use]
  pub fn upload_with_origin(
    self,
    queue: &gpu::Queue,
    source: &dyn WebGPU2DTextureSource,
    face: CubeTextureFace,
    mip_level: usize,
    origin: (usize, usize),
  ) -> Self {
    // validation
    queue.write_texture(
      gpu::TexelCopyTextureInfo {
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
      gpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    self
  }
}
