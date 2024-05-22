use std::{alloc::Layout, cell::UnsafeCell};

use crate::*;

/// improve the cache locality of given combination of components. This is as known as
/// the AOS (array of struct) storage type.
///
/// The access performance may not as good as static AOS because the memory access offset is
/// computed dynamically.
pub struct InterleavedDataContainer {
  pub inner: Arc<RwLock<InterleavedDataContainerInner>>,
  pub idx: usize,
}

pub struct InterleavedDataContainerInner {
  data: UnsafeCell<DynBuffer>,

  pub offsets: Vec<usize>,
  pub stride: usize,
  pub locks: Vec<Arc<RwLock<()>>>,
}

impl InterleavedDataContainerInner {
  pub fn un_init() -> Self {
    InterleavedDataContainerInner {
      data: UnsafeCell::new(DynBuffer::with_capacity(0, 0, 0)),
      offsets: Vec::new(),
      stride: 0,
      locks: Vec::new(),
    }
  }
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

    let mut new = Self::with_capacity((self.capacity * 2).max(new_len), self.align, self.stride);
    unsafe { std::ptr::copy_nonoverlapping(self.ptr, new.ptr, self.len * self.stride) }
    new.len = new_len;
    *self = new;
  }
}

unsafe impl Send for InterleavedDataContainerInner {}
unsafe impl Sync for InterleavedDataContainerInner {}

impl<T: CValue> ComponentStorage<T> for InterleavedDataContainer {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    let inner = self.inner.read();
    Box::new(InterleavedDataContainerReadView {
      phantom: PhantomData,
      offset: inner.offsets[self.idx],
      stride: inner.stride,
      data: self.inner.clone(),
      _guard: inner.locks[self.idx].make_read_holder(),
    })
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>> {
    let inner = self.inner.read();
    Box::new(InterleavedDataContainerReadWriteView {
      phantom: PhantomData,
      offset: inner.offsets[self.idx],
      stride: inner.stride,
      data: self.inner.clone(),
      _guard: inner.locks[self.idx].make_write_holder(),
    })
  }
}

#[derive(Clone)]
pub struct InterleavedDataContainerReadView<T> {
  phantom: PhantomData<T>,
  offset: usize,
  stride: usize,
  data: Arc<RwLock<InterleavedDataContainerInner>>,
  _guard: LockReadGuardHolder<()>,
}

impl<T: CValue> ComponentStorageReadView<T> for InterleavedDataContainerReadView<T> {
  fn get(&self, idx: RawEntityHandle) -> Option<&T> {
    let idx = idx.index() as usize;
    // todo generation check
    unsafe {
      let vec = (*self.data.data_ptr()).data.get();

      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
      Some(&*(address as *const T))
    }
  }
  fn get_without_generation_check(&self, idx: u32) -> Option<&T> {
    let idx = idx as usize;
    // todo generation check
    unsafe {
      let vec = (*self.data.data_ptr()).data.get();

      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
      Some(&*(address as *const T))
    }
  }

  fn clone_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    Box::new(self.clone())
  }
}

pub struct InterleavedDataContainerReadWriteView<T> {
  phantom: PhantomData<T>,
  offset: usize,
  stride: usize,
  data: Arc<RwLock<InterleavedDataContainerInner>>,
  _guard: LockWriteGuardHolder<()>,
}

impl<T: CValue> ComponentStorageReadWriteView<T> for InterleavedDataContainerReadWriteView<T> {
  fn get_mut(&mut self, idx: RawEntityHandle) -> Option<&mut T> {
    let idx = idx.index() as usize;
    // todo generation check
    unsafe {
      let vec = (*self.data.data_ptr()).data.get();
      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
      Some(&mut *(address as *mut T))
    }
  }

  fn get(&self, idx: RawEntityHandle) -> Option<&T> {
    let idx = idx.index() as usize;
    // todo generation check
    unsafe {
      let vec = (*self.data.data_ptr()).data.get();
      if idx >= (*vec).len {
        return None;
      }
      let address = (*vec).ptr as usize + self.stride * idx + self.offset;
      Some(&*(address as *mut T))
    }
  }

  unsafe fn grow_at_least(&mut self, max: usize) {
    let vec = (*self.data.data_ptr()).data.get();

    if (*vec).len <= max * self.stride {
      (*vec).resize((max + 1) * self.stride);
    }
  }
}

/// All interleave component has same allocation layout, so the must in same entity(E);

pub struct InterleavedDataContainerBuilder<E> {
  phantom: PhantomData<E>,
  layout: Option<Layout>,
  layouts: Vec<Layout>,
  offsets: Vec<usize>,
  ids: Vec<ComponentId>,
  shared: Arc<RwLock<InterleavedDataContainerInner>>,
  containers: Vec<Box<dyn Any>>,
}

impl<E> Default for InterleavedDataContainerBuilder<E> {
  fn default() -> Self {
    Self {
      phantom: Default::default(),
      layout: Default::default(),
      layouts: Default::default(),
      offsets: Default::default(),
      ids: Default::default(),
      shared: Arc::new(RwLock::new(InterleavedDataContainerInner::un_init())),
      containers: Default::default(),
    }
  }
}

impl<E> InterleavedDataContainerBuilder<E> {
  pub fn with_type<T>(&mut self) -> &mut Self
  where
    T: ComponentSemantic<Entity = E>,
  {
    let data = InterleavedDataContainer {
      inner: self.shared.clone(),
      idx: self.containers.len(),
    };
    self.containers.push(Box::new(
      Arc::new(data) as Arc<dyn ComponentStorage<T::Data>>
    ));
    self.with_type_impl(T::component_id(), Layout::new::<T::Data>())
  }
  pub fn with_type_impl(&mut self, id: ComponentId, new_layout: Layout) -> &mut Self {
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
  pub fn interleave_component_storages<E: EntitySemantic>(
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
      locks: std::iter::repeat_with(|| Arc::new(RwLock::new(())))
        .take(builder.layouts.len())
        .collect(),
    };
    *builder.shared.write() = buffer;

    // convert the old storage
    // todo check entity has any data
    self.access_ecg::<E, _>(|ecg| {
      let mut components = ecg.inner.inner.components.write();
      for (idx, container) in builder.containers.into_iter().enumerate() {
        let type_id = builder.ids[idx];
        let previous_storage = components.get_mut(&type_id).unwrap();
        // todo check com type

        previous_storage.inner.setup_new_storage(container);
      }
    });
    self
  }
}
