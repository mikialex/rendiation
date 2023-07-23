#![feature(specialization)]
#![feature(hash_raw_entry)]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]
#![allow(clippy::field_reassign_with_default)]

mod binding;
mod device;
mod encoder;
mod frame;
#[allow(hidden_glob_reexports)] // why method name shadows mod name??
mod pass;
mod pipeline;
mod queue;
mod read;
mod rendering;
mod resource;
mod surface;
mod types;

use std::{
  any::*,
  borrow::Cow,
  hash::{Hash, Hasher},
  ops::{Deref, DerefMut, Range},
  sync::atomic::{AtomicUsize, Ordering},
  sync::{Arc, RwLock},
};

use __core::fmt::Debug;
use __core::num::NonZeroUsize;
pub use binding::*;
use bytemuck::*;
pub use device::*;
use dyn_downcast::*;
pub use encoder::*;
use fast_hash_collection::*;
pub use frame::*;
use gpu::util::DeviceExt;
pub use gpu::*;
pub use pass::*;
pub use pipeline::*;
pub use queue::*;
pub use read::*;
pub use rendering::*;
use rendiation_texture_types::*;
pub use resource::*;
pub use surface::*;
use typed_arena::Arena;
pub use types::*;
use wgpu as gpu;

pub struct GPU {
  _instance: gpu::Instance,
  _adaptor: gpu::Adapter,
  pub device: GPUDevice,
  pub queue: GPUQueue,
}

impl GPU {
  pub fn poll(&self) {
    self._instance.poll_all(false);
  }

  pub async fn new() -> Self {
    let _instance = gpu::Instance::new(gpu::InstanceDescriptor {
      backends: gpu::Backends::PRIMARY,
      dx12_shader_compiler: Default::default(),
    });
    let power_preference = gpu::PowerPreference::HighPerformance;

    let _adaptor = _instance
      .request_adapter(&gpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: None,
        force_fallback_adapter: false,
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = _adaptor
      .request_device(
        &gpu::DeviceDescriptor {
          label: None,
          features: _adaptor.features(),
          limits: _adaptor.limits(),
        },
        None,
      )
      .await
      .expect("Unable to find a suitable GPU device!");

    let device = GPUDevice::new(device);
    let queue = GPUQueue::new(queue);

    Self {
      _instance,
      _adaptor,
      device,
      queue,
    }
  }
  pub async fn new_with_surface(surface_provider: &dyn SurfaceProvider) -> (Self, GPUSurface) {
    let _instance = gpu::Instance::new(gpu::InstanceDescriptor {
      backends: gpu::Backends::PRIMARY,
      dx12_shader_compiler: Default::default(),
    });
    let power_preference = gpu::PowerPreference::HighPerformance;

    let surface = surface_provider.create_surface(&_instance);
    let size = surface_provider.size();

    let _adaptor = _instance
      .request_adapter(&gpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = _adaptor
      .request_device(
        &gpu::DeviceDescriptor {
          label: None,
          features: _adaptor.features(),
          limits: _adaptor.limits(),
        },
        None,
      )
      .await
      .expect("Unable to find a suitable GPU device!");

    let device = GPUDevice::new(device);
    let queue = GPUQueue::new(queue);

    let surface = GPUSurface::new(&_adaptor, &device, surface, size);

    (
      Self {
        _instance,
        _adaptor,
        device,
        queue,
      },
      surface,
    )
  }

  pub fn create_encoder(&self) -> GPUCommandEncoder {
    self.device.create_encoder()
  }

  pub fn submit_encoder(&self, encoder: GPUCommandEncoder) {
    let cmb = encoder.finish();
    self.queue.submit(Some(cmb));
  }
}

pub trait IndexBufferSourceType: Pod {
  const FORMAT: gpu::IndexFormat;
}

impl IndexBufferSourceType for u32 {
  const FORMAT: gpu::IndexFormat = gpu::IndexFormat::Uint32;
}

impl IndexBufferSourceType for u16 {
  const FORMAT: gpu::IndexFormat = gpu::IndexFormat::Uint16;
}
