use wgpu::BufferAddress;

use super::{
  buffer::{has_copy_dst, Buffer},
  If, True,
};

pub struct Queue {
  queue: wgpu::Queue,
}

impl Queue {
  pub fn write_buffer<const SRC_USAGE: wgpu::BufferUsages>(
    &self,
    buffer: Buffer<SRC_USAGE>,
    offset: BufferAddress,
    data: &[u8],
  ) where
    If<{ has_copy_dst(SRC_USAGE) }>: True,
  {
    self.queue.write_buffer(&buffer.buffer, offset, data);
  }
}
