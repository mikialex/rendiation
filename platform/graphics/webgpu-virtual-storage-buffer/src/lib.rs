use std::sync::Arc;

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

pub struct StorageBufferMergeAllocator {
  buffer: Arc<RwLock<Option<GPUBufferResourceView>>>,
}

impl StorageBufferMergeAllocator {
  pub fn allocate<T: Std430MaybeUnsized>(
    &mut self,
    sub_buffer: StorageBufferInit<T>,
  ) -> SubMergedStorageBuffer<T> {
    todo!()
  }

  pub fn prepare(&mut self, gpu: &GPU) {
    //
  }
}

pub struct SubMergedStorageBuffer<T: ?Sized> {
  phantom: std::marker::PhantomData<T>,
  buffer: Arc<RwLock<Option<GPUBufferResourceView>>>,
}

impl<T: 'static> ShaderBindingProvider for SubMergedStorageBuffer<T> {
  type Node = ShaderStorageVirtualPtr<T>;
}

impl<T: ?Sized> CacheAbleBindingSource for SubMergedStorageBuffer<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    // self.gpu.get_binding_build_source()
    todo!()
  }
}
impl<T: ?Sized> BindableResourceView for SubMergedStorageBuffer<T> {
  fn as_bindable(&self) -> rendiation_webgpu::BindingResource {
    // self.gpu.as_bindable()
    // BindingResource::

    todo!()
  }
}

pub struct ShaderStorageVirtualPtr<T: ?Sized>(std::marker::PhantomData<T>);
pub type ShaderStorageVirtualNode<T> = Node<ShaderStorageVirtualPtr<T>>;

impl<T: 'static> ShaderNodeType for ShaderStorageVirtualPtr<T> {
  fn ty() -> ShaderValueType {
    todo!()
  }
}
