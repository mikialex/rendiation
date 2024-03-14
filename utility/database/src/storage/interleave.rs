use std::{alloc::Layout, cell::UnsafeCell};

use crate::*;

/// improve the cache locality of given combination of components. This is as known as
/// the AOS (array of struct) storage type.
///
/// The access performance is not as good as static AOS because the memory access offset is computed
/// dynamically.
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

/// The actual untyped storage buffer
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
      id: self.idx,
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
  id: usize,
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
      /// note, we only allow one write view to do resize. and when resizing, we need make sure
      /// the other component is nether write nor read, or it will cause a deadlock.
      use parking_lot::lock_api::RawRwLock;
      for (id, lock) in self.data.locks.iter().enumerate() {
        let lock = lock.raw();
        if id != self.id {
          lock.lock_exclusive()
        }
      }

      let vec = self.data.data.get();

      if (*vec).len <= max * self.stride {
        (*vec).resize((max + 1) * self.stride);
      }

      for (id, lock) in self.data.locks.iter().enumerate() {
        if id != self.id {
          lock.raw().unlock_exclusive();
        }
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

/// All interleave component has same allocation layout, so the must in same entity(E);

pub struct InterleavedDataContainerBuilder<E> {
  phantom: PhantomData<E>,
  layout: Option<Layout>,
  layouts: Vec<Layout>,
  offsets: Vec<usize>,
  ids: Vec<TypeId>,
}

impl<E> Default for InterleavedDataContainerBuilder<E> {
  fn default() -> Self {
    Self {
      phantom: Default::default(),
      layout: Default::default(),
      layouts: Default::default(),
      offsets: Default::default(),
      ids: Default::default(),
    }
  }
}

impl<E> InterleavedDataContainerBuilder<E> {
  pub fn with_type<T>(&mut self) -> &mut Self
  where
    T: ComponentSemantic<Entity = E>,
  {
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
  /// currently, we assume all storage is converted from the default storage type and no data
  /// exists.
  pub fn interleave_component_storages<E: Any>(
    self,
    build: impl FnOnce(
      &mut InterleavedDataContainerBuilder<E>,
    ) -> &mut InterleavedDataContainerBuilder<E>,
  ) -> Self {
    // collect the components that requires interleave
    let mut builder = InterleavedDataContainerBuilder::default();
    build(&mut builder);

    let combined_layout = builder.layout.unwrap();
    let stride = combined_layout.size();

    // create the underlayer storage
    let data = DynBuffer::with_capacity(0, combined_layout.align(), stride);
    let buffer = InterleavedDataContainerInner {
      data: UnsafeCell::new(data),
      offsets: builder.offsets,
      stride,
      locks: std::iter::repeat(Arc::new(RwLock::new(())))
        .take(builder.layouts.len())
        .collect(),
    };
    let buffer = Arc::new(buffer);

    // convert the old storage
    // todo check entity has any data
    self.access_ecg::<E, _>(|ecg| {
      let mut components = ecg.inner.components.write();
      for (idx, id) in builder.ids.iter().enumerate() {
        let previous_storage = components.get_mut(id).unwrap();
        // todo check com type

        let data = InterleavedDataContainer {
          inner: buffer.clone(),
          idx,
        };
        previous_storage.setup_new_storage(Box::new(Arc::new(data)));
      }
    });
    self
  }
}
