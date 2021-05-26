use wgpu::BufferAddress;

use super::{buffer::*, If, True, True2};

pub struct CommandEncoder {
  encoder: wgpu::CommandEncoder,
}

impl CommandEncoder {
  pub fn copy_buffer_to_buffer<
    const DST_USAGE: wgpu::BufferUsage,
    const SRC_USAGE: wgpu::BufferUsage,
  >(
    &mut self,
    source: &Buffer<SRC_USAGE>,
    source_offset: BufferAddress,
    destination: &Buffer<DST_USAGE>,
    destination_offset: BufferAddress,
    copy_size: BufferAddress,
  ) where
    If<{ has_copy_src(SRC_USAGE) }>: True,
    If<{ has_copy_dst(DST_USAGE) }>: True2,
  {
    self.encoder.copy_buffer_to_buffer(
      &source.buffer,
      source_offset,
      &destination.buffer,
      destination_offset,
      copy_size,
    );
  }
}
