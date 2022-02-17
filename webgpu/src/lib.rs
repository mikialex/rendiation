mod device;
mod encoder;
mod pass;
mod resource;
mod shadergraph_impl;
mod surface;
mod types;

pub use device::*;
pub use encoder::*;
pub use pass::*;
pub use resource::*;
pub use surface::*;
pub use types::*;

pub use binding::*;
mod binding;

pub use pipeline::*;
mod pipeline;

use bytemuck::*;
pub use wgpu::*;

use std::{
  cell::{Cell, RefCell},
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  marker::PhantomData,
  num::{NonZeroU32, NonZeroU8},
  ops::{Deref, DerefMut},
  rc::Rc,
  sync::atomic::{AtomicUsize, Ordering},
};

use rendiation_texture_types::*;
use wgpu::util::DeviceExt;

pub struct GPU {
  _instance: wgpu::Instance,
  _adaptor: wgpu::Adapter,
  pub device: GPUDevice,
  pub queue: wgpu::Queue,
  pub encoder: RefCell<GPUCommandEncoder>,
}

pub trait SurfaceProvider {
  fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface;
  fn size(&self) -> Size;
}

impl SurfaceProvider for winit::window::Window {
  fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface {
    unsafe { instance.create_surface(self) }
  }

  fn size(&self) -> Size {
    let size = self.inner_size();
    Size::from_u32_pair_min_one((size.width, size.height))
  }
}

impl GPU {
  pub async fn new() -> Self {
    let backend = wgpu::Backends::PRIMARY;
    let _instance = wgpu::Instance::new(backend);
    let power_preference = wgpu::PowerPreference::HighPerformance;

    let _adaptor = _instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: None,
        force_fallback_adapter: false,
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = _adaptor
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await
      .expect("Unable to find a suitable GPU device!");

    let device = GPUDevice::new(device);

    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: "Main GPU encoder".into(),
    });
    let encoder = GPUCommandEncoder::new(encoder, &device);

    let encoder = RefCell::new(encoder);

    Self {
      _instance,
      _adaptor,
      device,
      queue,
      encoder,
    }
  }
  pub async fn new_with_surface(surface_provider: &dyn SurfaceProvider) -> (Self, GPUSurface) {
    let backend = wgpu::Backends::all();
    let _instance = wgpu::Instance::new(backend);
    let power_preference = wgpu::PowerPreference::HighPerformance;

    let surface = surface_provider.create_surface(&_instance);
    let size = surface_provider.size();

    let _adaptor = _instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      })
      .await
      .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = _adaptor
      .request_device(
        &wgpu::DeviceDescriptor {
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

    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: "Main GPU encoder".into(),
    });
    let encoder = GPUCommandEncoder::new(encoder, &device);

    let encoder = RefCell::new(encoder);

    (
      Self {
        _instance,
        _adaptor,
        device,
        queue,
        encoder,
      },
      surface,
    )
  }

  pub fn submit(&self) {
    let encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: "Main GPU encoder".into(),
      });
    let mut encoder = GPUCommandEncoder::new(encoder, &self.device);

    let mut current_encoder = self.encoder.borrow_mut();
    let current_encoder: &mut GPUCommandEncoder = &mut current_encoder;

    std::mem::swap(current_encoder, &mut encoder);

    self.queue.submit(Some(encoder.finish()));
  }
}

pub trait IndexBufferSourceType: Pod {
  const FORMAT: wgpu::IndexFormat;
}

impl IndexBufferSourceType for u32 {
  const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

impl IndexBufferSourceType for u16 {
  const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

pub struct FrameTarget {
  pub size: Size,
  pub format: wgpu::TextureFormat,
  pub view: std::rc::Rc<wgpu::TextureView>,
}
