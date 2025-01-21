//! when wgpu used in web target, resource is not destroyed immediately when wgpu object dropped but
//! waiting for browser GC. If huge resource create and destroy in high frequency and due to the GC
//! uncertainty trigger time, huge memory will be consumed even OOM. In this case, we need to do
//! explicit destroy but not rely on wgpu's object drop to cleanup resource.
//!
//! see https://github.com/gfx-rs/wgpu/issues/4092

use std::mem::ManuallyDrop;

use crate::*;

pub trait ExplicitGPUResourceDestroy: 'static + Send + Sync {
  fn destroy(&self);
}

impl ExplicitGPUResourceDestroy for GPUBuffer {
  fn destroy(&self) {
    self.gpu.destroy()
  }
}

impl ExplicitGPUResourceDestroy for gpu::Texture {
  fn destroy(&self) {
    self.destroy()
  }
}

impl ExplicitGPUResourceDestroy for RawSampler {
  fn destroy(&self) {
    // no op
  }
}
impl ExplicitGPUResourceDestroy for gpu::TlasPackage {
  fn destroy(&self) {
    // no op
  }
}

#[derive(Clone, Default)]
pub struct DeferExplicitDestroy {
  to_drop: Arc<RwLock<Vec<Box<dyn ExplicitGPUResourceDestroy>>>>,
  in_recording_command_buffer_count: Arc<RwLock<usize>>,
}

impl DeferExplicitDestroy {
  pub fn new_command_buffer(&self) -> CommandBufferDeferExplicitDestroyFlusher {
    let mut count = self.in_recording_command_buffer_count.write().unwrap();
    let count: &mut usize = &mut count;
    *count += 1;
    CommandBufferDeferExplicitDestroyFlusher {
      inner: self.clone(),
    }
  }

  pub fn new_resource<T: ExplicitGPUResourceDestroy>(&self, r: T) -> ResourceExplicitDestroy<T> {
    ResourceExplicitDestroy {
      resource: ManuallyDrop::new(r),
      defer_drop: self.clone(),
    }
  }
}

pub struct ResourceExplicitDestroy<T: ExplicitGPUResourceDestroy> {
  resource: ManuallyDrop<T>,
  defer_drop: DeferExplicitDestroy,
}

impl<T: ExplicitGPUResourceDestroy> Deref for ResourceExplicitDestroy<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.resource
  }
}

impl<T: ExplicitGPUResourceDestroy> Drop for ResourceExplicitDestroy<T> {
  fn drop(&mut self) {
    let count = self
      .defer_drop
      .in_recording_command_buffer_count
      .read()
      .unwrap();
    let count: &usize = &count;
    let resource = unsafe { ManuallyDrop::take(&mut self.resource) };
    if *count != 0 {
      self
        .defer_drop
        .to_drop
        .write()
        .unwrap()
        .push(Box::new(resource));
    } else {
      resource.destroy();
    }
  }
}

pub struct CommandBufferDeferExplicitDestroyFlusher {
  inner: DeferExplicitDestroy,
}

impl Drop for CommandBufferDeferExplicitDestroyFlusher {
  fn drop(&mut self) {
    let mut count = self
      .inner
      .in_recording_command_buffer_count
      .write()
      .unwrap();
    let count: &mut usize = &mut count;
    *count -= 1;
    if *count == 0 {
      // we can safely flush here because the counter is protected by lock.
      let mut to_drop = self.inner.to_drop.write().unwrap();
      for i in to_drop.drain(..) {
        i.destroy();
      }
    }
  }
}
