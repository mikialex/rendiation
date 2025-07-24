use crate::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Facet)]
pub struct OffsetSize {
  pub offset: u32,
  pub size: u32,
}

impl OffsetSize {
  pub fn into_range(self) -> Range<usize> {
    self.offset as usize..(self.offset + self.size) as usize
  }
}

impl From<Range<u32>> for OffsetSize {
  fn from(value: Range<u32>) -> Self {
    Self {
      offset: value.start,
      size: value.len() as u32,
    }
  }
}

pub struct OffsetSizeBufferBuilder {
  current_offset: u32,
  buffer: Vec<OffsetSize>,
}

impl OffsetSizeBufferBuilder {
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      current_offset: 0,
      buffer: Vec::with_capacity(capacity),
    }
  }
  pub fn push_size(&mut self, size: u32) {
    self.buffer.push(OffsetSize {
      offset: self.current_offset,
      size,
    });
    self.current_offset += size;
  }

  pub fn finish(self) -> Vec<OffsetSize> {
    self.buffer
  }
}
