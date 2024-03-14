use std::{alloc::Layout, cell::UnsafeCell};

use crate::*;

/// improve the cache locality of given combination of components
pub struct InterleavedDataContainer {
  pub inner: Arc<InterleavedDataContainerInner>,
  pub idx: usize,
}

pub struct InterleavedDataContainerInner {
  data: UnsafeCell<DynBuffer>,

  pub offsets: Vec<usize>,
  pub stride: usize,
  pub locks: Vec<Arc<RwLock<()>>>,
}

struct DynBuffer {
  ptr: *mut u8,
  capacity: usize,
  len: usize,
  align: usize,
  stride: usize,
}

impl Drop for DynBuffer {
  fn drop(&mut self) {
    unsafe {
      if self.ptr.is_null() {
        return;
      }
      std::alloc::dealloc(
        self.ptr,
        Layout::from_size_align_unchecked(self.capacity * self.stride, self.align),
      );
    }
  }
}

impl DynBuffer {
  pub fn with_capacity(cap: usize, align: usize, stride: usize) -> Self {
    let ptr = if cap == 0 {
      std::ptr::null_mut()
    } else {
      unsafe { std::alloc::alloc(Layout::from_size_align_unchecked(cap * stride, align)) }
    };
    Self {
      ptr,
      capacity: cap,
      len: 0,
      align,
      stride,
    }
  }

  pub fn resize(&mut self, new_len: usize) {
    if self.len >= new_len {
      return;
    }

    if self.capacity >= new_len {
      self.len = new_len;
      return;
    }

    let new = Self::with_capacity((self.capacity * 2).max(new_len), self.align, self.stride);
    unsafe { std::ptr::copy_nonoverlapping(self.ptr, new.ptr, self.len * self.stride) }
    *self = new;
  }
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

      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
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
      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
      Some(&mut *(address as *mut T))
    }
  }

  fn grow_at_least(&mut self, max: usize) {
    unsafe {
      let vec = self.data.data.get();

      if (*vec).len <= max * self.stride {
        (*vec).resize((max + 1) * self.stride);
      }
    }
  }
}

impl<T> ComponentStorageReadView<T> for InterleavedDataContainerReadWriteView<T> {
  fn get(&self, idx: usize) -> Option<&T> {
    unsafe {
      let vec = self.data.data.get();

      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
      Some(&*(address as *const T))
    }
  }
}

#[derive(Default)]
pub struct InterleavedDataContainerBuilder {
  layout: Option<Layout>,
  layouts: Vec<Layout>,
  offsets: Vec<usize>,
  ids: Vec<TypeId>,
}

impl InterleavedDataContainerBuilder {
  pub fn with_type<T: Any>(&mut self) -> &mut Self {
    self.with_type_impl(TypeId::of::<T>(), Layout::new::<T>())
  }
  pub fn with_type_impl(&mut self, id: TypeId, new_layout: Layout) -> &mut Self {
    // todo,  zst supports
    if new_layout.size() == 0 {
      panic!("zst not supported")
    }

    self.layouts.push(new_layout);
    self.ids.push(id);
    if let Some(layout) = self.layout {
      let pad = layout.padding_needed_for(new_layout.align());
      self.offsets.push(layout.size() + pad);

      layout.extend(new_layout).unwrap();
    } else {
      self.offsets.push(0);
      self.layout = new_layout.into();
    }
    self
  }
}

impl Database {
  /// currently, we assume all storage is converted from the default storage type.
  pub fn interleave_component_storages(
    self,
    builder: impl FnOnce(&mut InterleavedDataContainerBuilder) -> &mut InterleavedDataContainerBuilder,
  ) -> Self {
    // let inner = self
    //   .component_storage
    //   .get(&ids[0])
    //   .unwrap()
    //   .create_read_write_view()
    //   .downcast::<InterleavedDataContainerInner>()

    self
  }
}
