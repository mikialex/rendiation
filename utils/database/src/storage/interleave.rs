use crate::*;

/// improve the cache locality of given combination of components
pub struct InterleavedDataContainer {
  pub inner: Arc<InterleavedDataContainerInner>,
  pub idx: usize,
}

pub struct InterleavedDataContainerInner {
  pub data: std::cell::UnsafeCell<Vec<u8>>,
  pub offsets: Vec<usize>,
  pub stride: usize,
  pub locks: Vec<Arc<RwLock<()>>>,
}

unsafe impl Send for InterleavedDataContainer {}
unsafe impl Sync for InterleavedDataContainer {}

impl<T: 'static> ComponentStorage<T> for InterleavedDataContainer {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    Box::new(InterleavedDataContainerReadView {
      phantom: PhantomData,
      offset: self.inner.offsets[self.idx],
      stride: self.inner.stride,
      data: self.inner.clone(),
      _guard: self.inner.locks[self.idx].make_read_holder(),
    })
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>> {
    Box::new(InterleavedDataContainerReadWriteView {
      phantom: PhantomData,
      offset: self.inner.offsets[self.idx],
      stride: self.inner.stride,
      data: self.inner.clone(),
      _guard: self.inner.locks[self.idx].make_write_holder(),
    })
  }
}

pub struct InterleavedDataContainerReadView<T> {
  phantom: PhantomData<T>,
  offset: usize,
  stride: usize,
  data: Arc<InterleavedDataContainerInner>,
  _guard: LockReadGuardHolder<()>,
}

impl<T> ComponentStorageReadView<T> for InterleavedDataContainerReadView<T> {
  fn get(&self, idx: usize) -> Option<&T> {
    unsafe {
      let vec = self.data.data.get();

      if idx >= (*vec).len() {
        return None;
      }
      let address = (*vec).as_ptr() as usize + self.stride * idx + self.offset;
      Some(&*(address as *const T))
    }
  }
}

pub struct InterleavedDataContainerReadWriteView<T> {
  phantom: PhantomData<T>,
  offset: usize,
  stride: usize,
  data: Arc<InterleavedDataContainerInner>,
  _guard: LockWriteGuardHolder<()>,
}

impl<T> ComponentStorageReadWriteView<T> for InterleavedDataContainerReadWriteView<T> {
  fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
    unsafe {
      let vec = self.data.data.get();
      if idx >= (*vec).len() {
        return None;
      }
      let address = (*vec).as_ptr() as usize + self.stride * idx + self.offset;
      Some(&mut *(address as *mut T))
    }
  }
}

impl<T> ComponentStorageReadView<T> for InterleavedDataContainerReadWriteView<T> {
  fn get(&self, idx: usize) -> Option<&T> {
    unsafe {
      let vec = self.data.data.get();

      if idx >= (*vec).len() {
        return None;
      }
      let address = (*vec).as_ptr() as usize + self.stride * idx + self.offset;
      Some(&*(address as *const T))
    }
  }
}

impl Database {
  pub fn interleave_component_storages(self, ids: impl IntoIterator<Item = TypeId>) -> Self {
    // let inner = self
    //   .component_storage
    //   .get(&ids[0])
    //   .unwrap()
    //   .create_read_write_view()
    //   .downcast::<InterleavedDataContainerInner>()

    self
  }
}
