use crate::*;

#[derive(Clone)]
pub enum MaybeCombinedStorageAllocator {
  Combined(CombinedStorageBufferAllocator),
  Default,
}

impl MaybeCombinedStorageAllocator {
  /// label must unique across binding
  pub fn new(label: impl Into<String>, enable_combine: bool, use_packed_layout: bool) -> Self {
    if enable_combine {
      Self::Combined(CombinedStorageBufferAllocator::new(
        label,
        use_packed_layout,
      ))
    } else {
      Self::Default
    }
  }

  pub fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    sub_buffer_u32_size: u32,
  ) -> BoxedAbstractStorageBuffer<T> {
    if let Self::Combined(combined) = self {
      Box::new(combined.allocate(sub_buffer_u32_size))
    } else {
      todo!()
    }
  }

  pub fn rebuild(&self, gpu: &GPU) {
    if let Self::Combined(combined) = self {
      combined.rebuild(gpu);
    }
  }
}
