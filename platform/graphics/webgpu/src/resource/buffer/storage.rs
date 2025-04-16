use crate::*;

pub struct StorageBufferReadonlyDataView<T: Std430MaybeUnsized + ?Sized> {
  pub gpu: GPUBufferResourceView,
  pub(crate) phantom: PhantomData<T>,
}

impl<T: Std430MaybeUnsized + ?Sized> StorageBufferReadonlyDataView<T> {
  pub fn try_from_raw(gpu: GPUBufferResourceView) -> Option<Self> {
    // todo, check if size is correct
    if gpu.resource.desc.usage.contains(gpu::BufferUsages::STORAGE) {
      Some(StorageBufferReadonlyDataView {
        gpu,
        phantom: PhantomData,
      })
    } else {
      None
    }
  }
  pub fn into_rw_view(self) -> StorageBufferDataView<T> {
    StorageBufferDataView {
      gpu: self.gpu.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: Std430> StorageBufferReadonlyDataView<[T]> {
  pub fn item_count(&self) -> u32 {
    let size: u64 = self.view_byte_size().into();
    let count = size / std::mem::size_of::<T>() as u64;
    count as u32
  }
}

impl<T: Std430MaybeUnsized + ?Sized> Clone for StorageBufferReadonlyDataView<T> {
  fn clone(&self) -> Self {
    Self {
      gpu: self.gpu.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: Std430MaybeUnsized + ?Sized> Deref for StorageBufferReadonlyDataView<T> {
  type Target = GPUBufferResourceView;

  fn deref(&self) -> &Self::Target {
    &self.gpu
  }
}

impl<T: Std430MaybeUnsized + ?Sized> BindableResourceProvider for StorageBufferReadonlyDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> CacheAbleBindingSource for StorageBufferReadonlyDataView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_binding_build_source()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> BindableResourceView for StorageBufferReadonlyDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

impl<T: Std430MaybeUnsized + ?Sized> StorageBufferReadonlyDataView<T> {
  pub fn create(device: &GPUDevice, data: &T) -> Self {
    Self::create_by(device, StorageBufferInit::WithInit(data))
  }

  pub fn create_by(device: &GPUDevice, source: StorageBufferInit<T>) -> Self {
    Self::create_by_with_extra_usage(device, source, gpu::BufferUsages::empty())
  }

  pub fn create_by_with_extra_usage(
    device: &GPUDevice,
    source: StorageBufferInit<T>,
    extra_usage: gpu::BufferUsages,
  ) -> Self {
    let usage = gpu::BufferUsages::STORAGE
      | gpu::BufferUsages::COPY_DST
      | gpu::BufferUsages::COPY_SRC
      | extra_usage;

    let init = source.into_buffer_init();
    let desc = GPUBufferDescriptor {
      size: init.size(),
      usage,
    };
    let gpu = GPUBuffer::create(device, init, usage);

    let gpu = GPUBufferResource::create_with_raw(gpu, desc, device).create_default_view();

    Self {
      gpu,
      phantom: PhantomData,
    }
  }

  // maybe here we should do sophisticated optimization to merge the adjacent writes.
  pub fn write_at(&self, offset: u64, data: &[u8], queue: &GPUQueue) {
    queue.write_buffer(&self.gpu.buffer.gpu, offset, data);
  }
}

/// just short convenient method
pub fn create_gpu_readonly_storage<T: Std430MaybeUnsized + ?Sized>(
  data: &T,
  device: impl AsRef<GPUDevice>,
) -> StorageBufferReadonlyDataView<T> {
  StorageBufferReadonlyDataView::create(device.as_ref(), data)
}

pub struct StorageBufferDataView<T: Std430MaybeUnsized + ?Sized> {
  pub gpu: GPUBufferResourceView,
  pub(crate) phantom: PhantomData<T>,
}

impl<T: Std430MaybeUnsized + ?Sized> Clone for StorageBufferDataView<T> {
  fn clone(&self) -> Self {
    Self {
      gpu: self.gpu.clone(),
      phantom: Default::default(),
    }
  }
}

impl<T: Std430MaybeUnsized + ?Sized> StorageBufferDataView<T> {
  pub fn try_from_raw(gpu: GPUBufferResourceView) -> Option<Self> {
    // todo, check if size is correct
    if gpu.resource.desc.usage.contains(gpu::BufferUsages::STORAGE) {
      Some(StorageBufferDataView {
        gpu,
        phantom: PhantomData,
      })
    } else {
      None
    }
  }
  pub fn into_readonly_view(self) -> StorageBufferReadonlyDataView<T> {
    StorageBufferReadonlyDataView {
      gpu: self.gpu.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: Std430> StorageBufferDataView<[T]> {
  pub fn item_count(&self) -> u32 {
    let size: u64 = self.view_byte_size().into();
    let count = size / std::mem::size_of::<T>() as u64;
    count as u32
  }
}

/// we are not suppose to transmute u32 atomic_u32 in host side, instead we transmute to marker
/// type,because the u32 atomic_u32 transmutation is not guaranteed to work on all platform due to
/// alignment difference. see: https://github.com/rust-lang/rust/issues/76314
///
/// todo, if other forms of transmutation is needed, we can add a unsafe escape for that.
impl<T> StorageBufferDataView<[T]>
where
  T: Std430 + AtomicityShaderNodeType,
{
  pub fn into_device_atomic_array(self) -> StorageBufferDataView<[DeviceAtomic<T>]> {
    StorageBufferDataView {
      gpu: self.gpu.clone(),
      phantom: PhantomData,
    }
  }
}
impl<T> StorageBufferDataView<[DeviceAtomic<T>]>
where
  T: Std430 + AtomicityShaderNodeType,
{
  pub fn into_host_nonatomic_array(self) -> StorageBufferDataView<[T]> {
    StorageBufferDataView {
      gpu: self.gpu.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: Std430MaybeUnsized + ?Sized> Deref for StorageBufferDataView<T> {
  type Target = GPUBufferResourceView;

  fn deref(&self) -> &Self::Target {
    &self.gpu
  }
}

impl<T: Std430MaybeUnsized + ?Sized> BindableResourceProvider for StorageBufferDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> CacheAbleBindingSource for StorageBufferDataView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_binding_build_source()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> BindableResourceView for StorageBufferDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

impl<'a, T: Std430> From<&'a [T]> for StorageBufferInit<'a, [T]> {
  fn from(value: &'a [T]) -> Self {
    StorageBufferInit::WithInit(value)
  }
}

#[derive(Clone, Copy)]
pub struct ZeroedArrayByArrayLength(pub usize);

impl<T: Std430> From<ZeroedArrayByArrayLength> for StorageBufferInit<'_, [T]> {
  fn from(len: ZeroedArrayByArrayLength) -> Self {
    let byte_len = std::mem::size_of::<T>() * len.0;
    StorageBufferInit::Zeroed(NonZeroU64::new(byte_len as u64).unwrap())
  }
}

/// just short convenient method
pub fn create_gpu_read_write_storage<'a, T: Std430MaybeUnsized + ?Sized + 'static>(
  data: impl Into<StorageBufferInit<'a, T>>,
  device: impl AsRef<GPUDevice>,
) -> StorageBufferDataView<T> {
  StorageBufferDataView::create_by(device.as_ref(), data.into())
}

pub struct StorageBufferSizedZeroed<T>(PhantomData<T>);
impl<T> Default for StorageBufferSizedZeroed<T> {
  fn default() -> Self {
    Self(Default::default())
  }
}
impl<T: Std430> From<StorageBufferSizedZeroed<T>> for StorageBufferInit<'_, T> {
  fn from(_: StorageBufferSizedZeroed<T>) -> Self {
    let byte_len = std::mem::size_of::<T>();
    StorageBufferInit::Zeroed(NonZeroU64::new(byte_len as u64).unwrap())
  }
}

pub enum StorageBufferInit<'a, T: Std430MaybeUnsized + ?Sized> {
  WithInit(&'a T),
  Zeroed(std::num::NonZeroU64),
}

impl<'a, T: Std430MaybeUnsized + ?Sized> StorageBufferInit<'a, T> {
  pub fn into_buffer_init(self) -> BufferInit<'a> {
    match self {
      StorageBufferInit::WithInit(data) => BufferInit::WithInit(data.bytes()),
      StorageBufferInit::Zeroed(size) => BufferInit::Zeroed(size),
    }
  }
}

impl<T: Std430MaybeUnsized + ?Sized> StorageBufferDataView<T> {
  pub fn create(device: &GPUDevice, data: &T) -> Self {
    Self::create_by(device, StorageBufferInit::WithInit(data))
  }

  pub fn create_by(device: &GPUDevice, source: StorageBufferInit<T>) -> Self {
    Self::create_by_with_extra_usage(device, source, gpu::BufferUsages::empty())
  }

  pub fn create_by_with_extra_usage(
    device: &GPUDevice,
    source: StorageBufferInit<T>,
    extra_usage: gpu::BufferUsages,
  ) -> Self {
    let usage = gpu::BufferUsages::STORAGE
      | gpu::BufferUsages::COPY_DST
      | gpu::BufferUsages::COPY_SRC
      | extra_usage;

    let init = source.into_buffer_init();
    let desc = GPUBufferDescriptor {
      size: init.size(),
      usage,
    };

    let gpu = GPUBuffer::create(device, init, usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, desc, device).create_default_view();

    Self {
      gpu,
      phantom: PhantomData,
    }
  }
}
