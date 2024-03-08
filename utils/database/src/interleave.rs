use crate::*;

/// improve the cache locality of given combination of components
pub struct InterleavedDataContainer {
  pub data: Arc<std::cell::UnsafeCell<Vec<u8>>>,
  // todo small vec
  pub offsets: Vec<usize>,
  pub locks: Vec<RwLock<()>>,
}

pub struct InterleavedDataContainerReadView<'a, T> {
  phantom: PhantomData<T>,
  field_idx: usize,
  source: &'a InterleavedDataContainer,
  _guard: parking_lot::RwLockReadGuard<'static, ()>,
}

impl<'a, T> InterleavedDataContainerReadView<'a, T> {
  pub fn get(&self, idx: usize) -> &T {
    unsafe {
      let vec = self.source.data.get();
      let address = (*vec).as_ptr() as usize + self.source.offsets[self.field_idx] * idx;
      &*(address as *const T)
    }
  }
}
