mod texture;
pub use texture::*;

mod buffer;
pub use buffer::*;

mod array;
pub use array::*;

mod sampler;
pub use sampler::*;

mod acceleration_structure;
pub use acceleration_structure::*;

mod defer_explicit_destroy;
pub use defer_explicit_destroy::*;

use crate::*;

pub struct ResourceViewContainer<T: Resource> {
  // when resource view is hold, the resource it self should keep existing
  pub resource: ResourceRc<T>,
  pub view: T::View,
  pub guid: usize,
  pub desc: T::ViewDescriptor,
}

impl<T: Resource> std::ops::Deref for ResourceViewContainer<T> {
  type Target = T::View;

  fn deref(&self) -> &Self::Target {
    &self.view
  }
}

/// store the resource with it's create parameter,
/// and some dropping callbacks
pub struct ResourceContainer<T: Resource> {
  _counter: Counted<Self>,
  pub guid: usize,
  pub resource: ResourceExplicitDestroy<T>,
  pub desc: T::Descriptor,
  pub(crate) bindgroup_holder: BindGroupResourceHolder,
}

impl<T: Resource> std::ops::Deref for ResourceContainer<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.resource
  }
}

static RESOURCE_GUID: AtomicUsize = AtomicUsize::new(0);
pub fn get_new_resource_guid() -> usize {
  RESOURCE_GUID.fetch_add(1, Ordering::Relaxed)
}

impl<T: Resource> ResourceContainer<T> {
  pub fn gpu_resource(&self) -> &T {
    &self.resource
  }

  pub fn create(desc: T::Descriptor, device: &GPUDevice) -> Self
  where
    T: InitResourceByAllocation,
  {
    let resource = T::create_resource(&desc, device);
    Self::create_with_raw(resource, desc, device)
  }

  pub fn create_with_source(source: T::Source, device: &GPUDevice) -> Self
  where
    T: InitResourceBySource,
  {
    let (resource, desc) = T::create_resource_with_source(&source, device);
    Self::create_with_raw(resource, desc, device)
  }

  pub fn create_with_raw(resource: T, desc: T::Descriptor, device: &GPUDevice) -> Self {
    Self {
      _counter: Default::default(),
      guid: get_new_resource_guid(),
      resource: device
        .inner
        .deferred_explicit_destroy
        .new_resource(resource),
      desc,
      bindgroup_holder: Default::default(),
    }
  }
}

pub trait Resource: 'static + Sized + ExplicitGPUResourceDestroy + Send + Sync {
  type Descriptor;
  type View;
  type ViewDescriptor;

  fn create_view(&self, des: &Self::ViewDescriptor) -> Self::View;
}

pub trait InitResourceByAllocation: Resource {
  fn create_resource(des: &Self::Descriptor, device: &GPUDevice) -> Self;
}

pub trait InitResourceBySource: Resource {
  type Source;
  fn create_resource_with_source(
    source: &Self::Source,
    device: &GPUDevice,
  ) -> (Self, Self::Descriptor);
}

pub struct ResourceRc<T: Resource> {
  inner: Arc<ResourceContainer<T>>,
}

impl<T: Resource> std::fmt::Debug for ResourceRc<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("ResourceViewRc").finish()
  }
}

impl<T: Resource> PartialEq for ResourceRc<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

impl<T: Resource> std::ops::Deref for ResourceRc<T> {
  type Target = ResourceContainer<T>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T: Resource> Clone for ResourceRc<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct ResourceViewRc<T: Resource> {
  inner: Arc<ResourceViewContainer<T>>,
}

impl<T: Resource> std::fmt::Debug for ResourceViewRc<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("ResourceViewRc").finish()
  }
}

impl<T: Resource> PartialEq for ResourceViewRc<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

impl<T: Resource> std::ops::Deref for ResourceViewRc<T> {
  type Target = ResourceViewContainer<T>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T: Resource> Clone for ResourceViewRc<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> BindableResourceView for ResourceViewRc<T>
where
  T::View: BindableResourceView,
  T: Resource,
{
  fn as_bindable(&self) -> gpu::BindingResource {
    self.inner.as_bindable()
  }
}

static RESOURCE_VIEW_GUID: AtomicUsize = AtomicUsize::new(0);
pub fn get_resource_view_guid() -> usize {
  RESOURCE_VIEW_GUID.fetch_add(1, Ordering::Relaxed)
}

impl<T: Resource> ResourceRc<T> {
  pub fn gpu_resource(&self) -> &T {
    &self.resource
  }

  #[must_use]
  pub fn create(desc: T::Descriptor, device: &GPUDevice) -> Self
  where
    T: InitResourceByAllocation,
  {
    Self {
      inner: Arc::new(ResourceContainer::create(desc, device)),
    }
  }

  pub fn create_with_raw(resource: T, desc: T::Descriptor, device: &GPUDevice) -> Self {
    Self {
      inner: Arc::new(ResourceContainer::create_with_raw(resource, desc, device)),
    }
  }

  pub fn create_with_source(source: T::Source, device: &GPUDevice) -> Self
  where
    T: InitResourceBySource,
  {
    Self {
      inner: Arc::new(ResourceContainer::create_with_source(source, device)),
    }
  }

  pub fn create_view(&self, desc: T::ViewDescriptor) -> ResourceViewRc<T> {
    let view = self.inner.resource.create_view(&desc);
    let inner = ResourceViewContainer {
      resource: self.clone(),
      view,
      guid: get_resource_view_guid(),
      desc,
    };
    ResourceViewRc {
      inner: Arc::new(inner),
    }
  }

  pub fn create_default_view(&self) -> ResourceViewRc<T>
  where
    T::ViewDescriptor: Default,
  {
    self.create_view(Default::default())
  }
}
