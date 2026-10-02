use crate::*;

/// The R32Uint texture that stores the u32 heap in row major order.
///
/// The first texel is the header, it stores the array length if the content type is a runtime
/// sized array. The header is only written by the queue, and the copies recorded in encoder never
/// touch it, so the header writes always take effect in the issue order, even if the encoder that
/// contains the copies is submitted later.
#[derive(Clone)]
pub(crate) struct TextureU32Heap {
  pub view: GPUTypedTextureView<TextureDimension2, u32>,
}

impl TextureU32Heap {
  /// the content is zero initialized
  pub fn new(extent: TexelExtent, label: &str, device: &GPUDevice) -> Self {
    let desc: raw_gpu::TextureDescriptor<'static> = TextureDescriptor {
      label: None,
      size: Extent3d {
        width: extent.width,
        height: extent.height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::R32Uint,
      usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
      view_formats: &[],
    };
    let raw = device.create_texture(&desc.map_label(|_| Some(label)));
    let texture = GPUTexture::create_with_raw(raw, desc, device);

    Self {
      view: texture.create_default_view().try_into().unwrap(),
    }
  }

  pub fn extent(&self) -> TexelExtent {
    let size = self.view.resource.desc.size;
    TexelExtent {
      width: size.width,
      height: size.height,
    }
  }

  fn gpu_texture(&self) -> &raw_gpu::Texture {
    self.view.resource.gpu_resource()
  }

  /// write the data to the linear texel range that starts at `texel_offset`
  pub fn write(&self, queue: &GPUQueue, texel_offset: u64, data: &[u8]) {
    assert!(data.len().is_multiple_of(4));
    let count = data.len() as u64 / 4;
    assert!(texel_offset + count <= self.extent().texel_count());

    let texture = self.gpu_texture();
    split_linear_range(texel_offset, self.extent().width, count, |rect| {
      let start = rect.linear_offset as usize * 4;
      let len = rect.width as usize * rect.height as usize * 4;
      queue.write_texture(
        texel_copy_info(texture, rect.dst),
        &data[start..start + len],
        TexelCopyBufferLayout {
          offset: 0,
          // write_texture does not require the row alignment
          bytes_per_row: Some(rect.width * 4),
          rows_per_image: None,
        },
        rect_extent(&rect),
      );
    });
  }

  /// record the copy of the linear texel range from self to target, the target must not be self
  pub fn copy_to(
    &self,
    target: &Self,
    src_texel_offset: u64,
    dst_texel_offset: u64,
    count: u64,
    encoder: &mut GPUCommandEncoder,
  ) {
    assert!(src_texel_offset + count <= self.extent().texel_count());
    assert!(dst_texel_offset + count <= target.extent().texel_count());

    let src = self.gpu_texture();
    let dst = target.gpu_texture();
    split_linear_copy(
      src_texel_offset,
      self.extent().width,
      dst_texel_offset,
      target.extent().width,
      count,
      |rect| {
        encoder.copy_texture_to_texture(
          texel_copy_info(src, rect.src),
          texel_copy_info(dst, rect.dst),
          rect_extent(&rect),
        );
      },
    );
  }
}

fn texel_copy_info(
  texture: &raw_gpu::Texture,
  (x, y): (u32, u32),
) -> raw_gpu::TexelCopyTextureInfo<'_> {
  TexelCopyTextureInfo {
    texture,
    mip_level: 0,
    origin: Origin3d { x, y, z: 0 },
    aspect: TextureAspect::All,
  }
}

fn rect_extent(rect: &TexelCopyRect) -> Extent3d {
  Extent3d {
    width: rect.width,
    height: rect.height,
    depth_or_array_layers: 1,
  }
}
