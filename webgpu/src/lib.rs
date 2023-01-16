#![feature(specialization)]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]

mod device;
mod encoder;
mod pass;
mod read;
mod rendering;
mod resource;
mod surface;
mod types;

pub use device::*;
pub use encoder::*;
pub use pass::*;
pub use read::*;
pub use rendering::*;
pub use resource::*;
pub use surface::*;
pub use types::*;

pub use binding::*;
mod binding;

pub use pipeline::*;
mod pipeline;

use bytemuck::*;
pub use gpu::*;
use wgpu as gpu;

use __core::fmt::Debug;
use __core::num::NonZeroUsize;
use std::{
  any::*,
  borrow::Cow,
  cell::{Cell, RefCell},
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  num::{NonZeroU32, NonZeroU8},
  ops::{Deref, DerefMut, Range},
  rc::Rc,
  sync::atomic::{AtomicUsize, Ordering},
};

use gpu::util::DeviceExt;
use rendiation_texture_types::*;
use typed_arena::Arena;

pub struct GPU {
  _instance: gpu::Instance,
  _adaptor: gpu::Adapter,
  pub device: GPUDevice,
  pub queue: gpu::Queue,
}

impl GPU {
  pub fn poll(&self) {
    self._instance.poll_all(false);
  }

  pub async fn new() -> Self {
    let backend = gpu::Backends::PRIMARY;
    let _instance = gpu::Instance::new(backend);
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
      .request_device(&gpu::DeviceDescriptor::default(), None)
      .await
      .expect("Unable to find a suitable GPU device!");

    let device = GPUDevice::new(device);

    Self {
      _instance,
      _adaptor,
      device,
      queue,
    }
  }
  pub async fn new_with_surface(surface_provider: &dyn SurfaceProvider) -> (Self, GPUSurface) {
    let backend = gpu::Backends::all();
    let _instance = gpu::Instance::new(backend);
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
    let encoder = self
      .device
      .create_command_encoder(&gpu::CommandEncoderDescriptor { label: None });
    GPUCommandEncoder::new(encoder, &self.device)
  }

  pub fn submit_encoder(&self, encoder: GPUCommandEncoder) {
    let cmb = encoder.finish();
    cmb.on_submit.resolve();

    self.queue.submit(Some(cmb.gpu));
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
