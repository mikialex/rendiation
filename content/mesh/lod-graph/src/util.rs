use crate::*;

#[derive(Clone, Copy)]
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
