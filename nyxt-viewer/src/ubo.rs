use rendiation_ral::UniformHandle;

use crate::{NyxtViewerHandle, NyxtViewerInner, NyxtViewerMutableHandle, GFX};

#[derive(Copy, Clone)]
pub struct UniformHandleWrap<T>(UniformHandle<GFX, T>);

impl<T: Copy + 'static> NyxtViewerHandle for UniformHandleWrap<T> {
  type Item = T;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.resource.bindable.uniform_buffers.get_data(self.0)
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.resource.bindable.uniform_buffers.delete(self.0)
  }
}
impl<T: Copy + 'static> NyxtViewerMutableHandle for UniformHandleWrap<T> {
  fn get_mut(self, inner: &mut NyxtViewerInner) -> &mut Self::Item {
    inner.resource.bindable.uniform_buffers.mutate(self.0)
  }
}
