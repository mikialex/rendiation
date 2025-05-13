#![feature(type_alias_impl_trait)]

mod binding;
mod device;
mod encoder;
mod frame;
mod instance_poller;
mod pass;
mod pipeline;
mod query;
mod queue;
mod read;
mod rendering;
mod resource;
mod surface;
mod types;

use core::fmt::Debug;
use core::num::NonZeroUsize;
use core::{marker::PhantomData, num::NonZeroU64};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::{
  any::*,
  borrow::Cow,
  hash::{Hash, Hasher},
  ops::{Deref, DerefMut, Range},
  sync::atomic::{AtomicUsize, Ordering},
  sync::Arc,
};

pub use binding::*;
use bytemuck::*;
pub use device::*;
pub use encoder::*;
use fast_hash_collection::*;
pub use frame::*;
use futures::{Future, FutureExt};
use gpu::RenderPassTimestampWrites;
// note: we can not just use * because it cause core conflict
pub use gpu::{
  util, util::DeviceExt, vertex_attr_array, AccelerationStructureFlags,
  AccelerationStructureGeometryFlags, AccelerationStructureUpdateMode, AddressMode, Backends,
  BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
  BindingResource, Blas, BlasBuildEntry, BlasGeometries, BlasGeometrySizeDescriptors,
  BlasTriangleGeometry, BlasTriangleGeometrySizeDescriptor, Buffer, BufferAsyncError, Color,
  CommandEncoder, CompareFunction, CreateSurfaceError, Device, Features, FilterMode, FragmentState,
  IndexFormat, Limits, LoadOp, Operations, PipelineLayoutDescriptor, PowerPreference, Queue,
  RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
  RequestDeviceError, Sampler, SamplerBorderColor, SamplerDescriptor, ShaderModuleDescriptor,
  ShaderSource, ShaderStages, StoreOp, SurfaceError, SurfaceTexture, TextureView,
  TextureViewDescriptor, Tlas, TlasBuildEntry, TlasInstance, TlasPackage, VertexBufferLayout,
  VertexState,
};
use heap_tools::*;
use instance_poller::GPUInstance;
use parking_lot::RwLock;
pub use pass::*;
pub use pipeline::*;
pub use query::*;
pub use queue::*;
pub use read::*;
pub use rendering::*;
use rendiation_shader_api::*;
use rendiation_shader_api::{Std430, Std430MaybeUnsized};
pub use rendiation_texture_types::*;
pub use resource::*;
use reuse_pool::*;
pub use surface::*;
pub use types::*;
use wgpu as gpu;
pub use wgpu_types::*;

#[derive(Clone)]
pub struct GPU {
  pub instance: GPUInstance,
  _adaptor: Arc<gpu::Adapter>,
  pub info: GPUInfo,
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

impl Default for GPUCreateConfig<'_> {
  fn default() -> Self {
    Self {
      backends: Backends::all(),
      power_preference: PowerPreference::HighPerformance,
      surface_for_compatible_check_init: None,
      minimal_required_features: Features::empty(),
      minimal_required_limits: Default::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct GPUInfo {
  pub adaptor_info: AdapterInfo,
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
  /// in some backend for example WebGL, the surface is required to create the instance, we have to
  /// return the init surface with the gpu itself
  pub async fn new(
    config: GPUCreateConfig<'_>,
  ) -> Result<(Self, Option<GPUSurface>), GPUCreateFailure> {
    let instance = gpu::Instance::new(&gpu::InstanceDescriptor {
      backends: config.backends,
      flags: Default::default(),
      backend_options: Default::default(),
    });
    let power_preference = gpu::PowerPreference::HighPerformance;

    let init_surface = config
      .surface_for_compatible_check_init
      .map(|s| s.0.create_surface(&instance))
      .transpose()?;

    let _adaptor = instance
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
          memory_hints: MemoryHints::Performance,
        },
        None,
      )
      .await?;

    let device = GPUDevice::new(device);
    let queue = GPUQueue::new(queue);

    let info = GPUInfo {
      adaptor_info: _adaptor.get_info(),
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

    let instance = GPUInstance::new(instance);

    let gpu = Self {
      instance,
      _adaptor: Arc::new(_adaptor),
      info,
      device,
      queue,
    };

    Ok((gpu, surface))
  }

  pub fn info(&self) -> &GPUInfo {
    &self.info
  }

  pub fn poll(&self, force_wait: bool) {
    self.instance.poll_all(force_wait);
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
    let surface = provider.create_surface(&self.instance)?;
    Ok(GPUSurface::new(
      &self._adaptor,
      &self.device,
      surface,
      init_resolution,
    ))
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

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DrawIndexedIndirect {
  /// The number of vertices to draw.
  pub vertex_count: u32,
  /// The number of instances to draw.
  pub instance_count: u32,
  /// The base index within the index buffer.
  pub base_index: u32,
  /// The value added to the vertex index before indexing into the vertex buffer.
  pub vertex_offset: i32,
  /// The instance ID of the first instance to draw.
  /// Has to be 0, unless INDIRECT_FIRST_INSTANCE is enabled.
  pub base_instance: u32,
}

impl DrawIndexedIndirect {
  pub fn new(
    vertex_count: u32,
    instance_count: u32,
    base_index: u32,
    vertex_offset: i32,
    base_instance: u32,
  ) -> Self {
    Self {
      vertex_count,
      instance_count,
      base_index,
      vertex_offset,
      base_instance,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DrawIndirect {
  /// The number of vertices to draw.
  pub vertex_count: u32,
  /// The number of instances to draw.
  pub instance_count: u32,
  /// The Index of the first vertex to draw.
  pub first_vertex: u32,
  /// The instance ID of the first instance to draw.
  ///
  /// Has to be 0, INDIRECT_FIRST_INSTANCE is enabled.
  pub first_instance: u32,
}

/// this fn is to replace the usage of `TextureUsages::all()` because not every fmt support
/// `TextureUsages::STORAGE_ATOMIC` and this will cause validation error.
pub fn basic_texture_usages() -> TextureUsages {
  let mut full = TextureUsages::all();
  full.remove(TextureUsages::STORAGE_ATOMIC);
  full
}
