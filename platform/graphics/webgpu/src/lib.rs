#![feature(hash_raw_entry)]
#![feature(type_alias_impl_trait)]

mod binding;
mod device;
mod encoder;
mod frame;
mod pass;
mod pipeline;
mod queue;
mod read;
mod rendering;
mod resource;
mod surface;
mod types;

use core::fmt::Debug;
use core::num::NonZeroUsize;
use std::{
  any::*,
  borrow::Cow,
  hash::{Hash, Hasher},
  ops::{Deref, DerefMut, Range},
  sync::atomic::{AtomicUsize, Ordering},
  sync::{Arc, RwLock},
};

pub use binding::*;
use bytemuck::*;
pub use device::*;
use dyn_downcast::*;
pub use encoder::*;
use fast_hash_collection::*;
pub use frame::*;
pub use gpu::Features;
// note: we can not just use * because it cause core conflict
pub use gpu::{
  util, util::DeviceExt, vertex_attr_array, AddressMode, Backends, BindGroup, BindGroupDescriptor,
  BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindingResource, Buffer,
  BufferAsyncError, Color, CommandEncoder, CompareFunction, CreateSurfaceError, Device, FilterMode,
  FragmentState, IndexFormat, Limits, LoadOp, Operations, PipelineLayoutDescriptor,
  PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
  RenderPipelineDescriptor, RequestDeviceError, Sampler, SamplerBorderColor, SamplerDescriptor,
  ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp, TextureView, TextureViewDescriptor,
  VertexBufferLayout, VertexState,
};
pub use pass::*;
pub use pipeline::*;
pub use queue::*;
pub use read::*;
pub use rendering::*;
use rendiation_shader_api::*;
use rendiation_texture_types::*;
pub use resource::*;
pub use surface::*;
use typed_arena::Arena;
pub use types::*;
use wgpu as gpu;
pub use wgpu_types::*;

pub struct GPU {
  _instance: gpu::Instance,
  _adaptor: gpu::Adapter,
  info: GPUInfo,
  pub device: GPUDevice,
  pub queue: GPUQueue,
}

pub struct GPUCreateConfig<'a> {
  pub backends: Backends,
  pub power_preference: PowerPreference,
  pub surface_for_compatible_check_init: Option<(&'a (dyn SurfaceProvider + 'a), Size)>,
  pub minimal_required_features: Features,
  pub minimal_required_limits: Limits,
}

impl<'a> Default for GPUCreateConfig<'a> {
  fn default() -> Self {
    let mut minimal_required_features = Features::all_webgpu_mask();
    minimal_required_features.remove(Features::TIMESTAMP_QUERY); // note: on macos we currently do not have this

    Self {
      backends: Backends::all(),
      power_preference: PowerPreference::HighPerformance,
      surface_for_compatible_check_init: None,
      minimal_required_features,
      minimal_required_limits: Default::default(),
    }
  }
}

#[derive(Clone)]
pub struct GPUInfo {
  pub requested_backend_type: Backends,
  pub power_preference: PowerPreference,
  pub supported_features: Features,
  pub supported_limits: Limits,
}

#[derive(thiserror::Error, Debug)]
pub enum GPUCreateFailure {
  #[error("Failed to request adapter, reasons unknown")]
  AdapterRequestFailed,
  #[error("Failed to request adapter, because failed to create test compatible surface")]
  AdapterRequestFailedByUnableCreateTestCompatibleSurface(#[from] CreateSurfaceError),
  #[error(
    "Failed to create device because the the adaptor can not meet the minimal feature requirement"
  )]
  UnableToMeetFeatureMinimalRequirement(Features),
  #[error(
    "Failed to create device because the the adaptor can not meet the minimal limit requirement"
  )]
  UnableToMeetLimitMinimalRequirement(Limits),
  #[error("Failed to create device, reasons unknown")]
  DeviceQueueCreateFailedUnknownReason(#[from] RequestDeviceError),
}

impl GPU {
  /// in some backend the surface is used to create the instance for example webgl, we have to
  /// return the init surface with the gpu itself
  pub async fn new(
    config: GPUCreateConfig<'_>,
  ) -> Result<(Self, Option<GPUSurface>), GPUCreateFailure> {
    let _instance = gpu::Instance::new(gpu::InstanceDescriptor {
      backends: config.backends,
      dx12_shader_compiler: Default::default(),
      flags: Default::default(),
      gles_minor_version: Default::default(),
    });
    let power_preference = gpu::PowerPreference::HighPerformance;

    let init_surface = config
      .surface_for_compatible_check_init
      .map(|s| s.0.create_surface(&_instance))
      .transpose()?;

    let _adaptor = _instance
      .request_adapter(&gpu::RequestAdapterOptions {
        power_preference,
        compatible_surface: init_surface.as_ref(),
        force_fallback_adapter: false,
      })
      .await
      .ok_or(GPUCreateFailure::AdapterRequestFailed)?;

    let supported_features = _adaptor.features();
    let supported_limits = _adaptor.limits();

    if !config
      .minimal_required_limits
      .check_limits(&supported_limits)
    {
      // todo, list unsatisfied limits
      return Err(GPUCreateFailure::UnableToMeetLimitMinimalRequirement(
        supported_limits,
      ));
    }
    if !supported_features.contains(config.minimal_required_features) {
      return Err(GPUCreateFailure::UnableToMeetFeatureMinimalRequirement(
        config.minimal_required_features - supported_features,
      ));
    }

    let (device, queue) = _adaptor
      .request_device(
        &gpu::DeviceDescriptor {
          label: None,
          required_features: supported_features,
          required_limits: supported_limits.clone(),
        },
        None,
      )
      .await?;

    let device = GPUDevice::new(device);
    let queue = GPUQueue::new(queue);

    let info = GPUInfo {
      requested_backend_type: config.backends,
      power_preference: config.power_preference,
      supported_features,
      supported_limits,
    };

    let surface = init_surface.map(|init_surface| {
      GPUSurface::new(
        &_adaptor,
        &device,
        init_surface,
        config.surface_for_compatible_check_init.as_ref().unwrap().1,
      )
    });

    let gpu = Self {
      _instance,
      _adaptor,
      info,
      device,
      queue,
    };

    Ok((gpu, surface))
  }

  pub fn poll(&self) {
    self._instance.poll_all(false);
  }

  pub fn create_cache_report(&self) -> GPUResourceCacheSizeReport {
    self.device.create_cache_report()
  }

  /// clear the resource cached in device. note,if the outside hold the cache, they may still not be
  /// released.
  pub fn clear_resource_cache(&self) {
    self.device.clear_resource_cache();
  }

  pub fn create_another_surface<'a>(
    &self,
    provider: &'a dyn SurfaceProvider,
    init_resolution: Size,
  ) -> Result<GPUSurface<'a>, CreateSurfaceError> {
    let surface = provider.create_surface(&self._instance)?;
    Ok(GPUSurface::new(
      &self._adaptor,
      &self.device,
      surface,
      init_resolution,
    ))
  }

  pub fn info(&self) -> &GPUInfo {
    &self.info
  }

  pub fn create_encoder(&self) -> GPUCommandEncoder {
    self.device.create_encoder()
  }

  pub fn submit_encoder(&self, encoder: GPUCommandEncoder) {
    self.queue.submit_encoder(encoder);
  }
}

impl AsRef<GPUDevice> for GPU {
  fn as_ref(&self) -> &GPUDevice {
    &self.device
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
