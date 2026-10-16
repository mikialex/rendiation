mod atomic_image_downgrade;
mod binding;
mod device;
mod encoder;
mod frame;
mod indirect;
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
use std::mem::ManuallyDrop;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::{
  any::*,
  borrow::Cow,
  hash::{Hash, Hasher},
  ops::{Deref, DerefMut, Range},
  sync::Arc,
  sync::atomic::{AtomicUsize, Ordering},
};

pub use atomic_image_downgrade::*;
pub use binding::*;
use bytemuck::*;
pub use device::*;
use dyn_clone::DynClone;
pub use encoder::*;
use event_source::*;
use fast_hash_collection::*;
pub use frame::*;
use futures::{Future, FutureExt};
use gpu::RenderPassTimestampWrites;
// note: we can not just use * because it will cause core conflict
pub use gpu::{
  AccelerationStructureFlags, AccelerationStructureGeometryFlags, AccelerationStructureUpdateMode,
  AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
  BindGroupLayoutDescriptor, BindingResource, Blas, BlasBuildEntry, BlasGeometries,
  BlasGeometrySizeDescriptors, BlasTriangleGeometry, BlasTriangleGeometrySizeDescriptor, Buffer,
  BufferAsyncError, Color, CommandEncoder, CompareFunction, CreateSurfaceError, Device, Features,
  FilterMode, FragmentState, IndexFormat, Limits, LoadOp, Operations, PipelineLayoutDescriptor,
  PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
  RenderPipelineDescriptor, RequestDeviceError, Sampler, SamplerBorderColor, SamplerDescriptor,
  ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp, SurfaceTexture, TextureView,
  TextureViewDescriptor, Tlas, TlasInstance, VertexBufferLayout, VertexState, util,
  util::DeviceExt, vertex_attr_array,
};
use heap_tools::*;
use hook::*;
pub use indirect::*;
pub use instance_poller::GPUInstance;
use parking_lot::RwLock;
pub use pass::*;
pub use pipeline::*;
pub use query::*;
pub use queue::*;
pub use read::*;
pub use rendering::*;
use rendiation_shader_api::*;
use rendiation_shader_api::{Std430, Std430MaybeUnsized};
use rendiation_shader_library::*;
pub use rendiation_texture_types::*;
pub use resource::*;
use reuse_pool::*;
pub use surface::*;
pub use types::*;
use wgpu as gpu;
pub use wgpu as raw_gpu;
pub use wgpu_types::*;

#[derive(Clone)]
pub struct GPU {
  pub instance: GPUInstance,
  pub adaptor: gpu::Adapter,
  pub info: Arc<GPUInfo>,
  pub device: GPUDevice,
  pub queue: GPUQueue,
}

pub struct GPUInstanceConfig {
  pub backends: Backends,
  /// if None, then using wgpu default behavior (on when debug build)
  pub enable_debug_info: Option<bool>,
  /// if None, then using wgpu default behavior (on when debug build)
  pub enable_backend_validation: Option<bool>,
  /// the dxc dll path for dx12 backend, the dll must support shader model 6.7 at least
  /// if None, then using fxc compiler, which is buggy.
  pub dx_compiler_dll_path: Option<String>,
  pub display: Option<Box<dyn WgpuHasDisplayHandle>>,
}

impl Default for GPUInstanceConfig {
  fn default() -> Self {
    Self {
      backends: Backends::all(),
      enable_backend_validation: None,
      enable_debug_info: None,
      dx_compiler_dll_path: None,
      display: None,
    }
  }
}

pub struct GPUDeviceConfig {
  pub power_preference: PowerPreference,
  pub minimal_required_features: Features,
  pub minimal_required_limits: Limits,
  pub default_shader_checks: ShaderRuntimeChecks,
}

impl Default for GPUDeviceConfig {
  fn default() -> Self {
    Self {
      power_preference: PowerPreference::HighPerformance,
      minimal_required_features: Features::empty(),
      minimal_required_limits: Default::default(),
      default_shader_checks: ShaderRuntimeChecks::checked(),
    }
  }
}

#[derive(Default)]
pub struct GPUCreateConfig {
  pub instance: GPUInstanceConfig,
  pub device: GPUDeviceConfig,
}

#[derive(thiserror::Error, Debug)]
pub enum GPUCreateFailure {
  #[error("Failed to request adapter")]
  AdapterRequestFailed(#[from] RequestAdapterError),
  #[error("Failed to request adapter, note WebGL backend requires a compatible surface")]
  AdapterRequestFailedWithoutCompatibleSurface(RequestAdapterError),
  #[error(
    "Failed to create device because the the adaptor can not meet the minimal feature requirement"
  )]
  UnableToMeetFeatureMinimalRequirement(Box<Features>),
  #[error(
    "Failed to create device because the the adaptor can not meet the minimal limit requirement"
  )]
  UnableToMeetLimitMinimalRequirement(Box<Limits>),
  #[error("Failed to create device, reasons unknown")]
  DeviceQueueCreateFailedUnknownReason(#[from] RequestDeviceError),
}

impl GPUInstance {
  pub fn create(config: GPUInstanceConfig) -> Self {
    let instance = gpu::Instance::new(gpu::InstanceDescriptor {
      backends: config.backends,
      flags: {
        let mut r = InstanceFlags::default();
        if let Some(validation) = config.enable_backend_validation {
          if validation {
            r.insert(InstanceFlags::VALIDATION)
          } else {
            r.remove(InstanceFlags::VALIDATION)
          }
        }
        if let Some(enable) = config.enable_debug_info {
          if enable {
            r.insert(InstanceFlags::DEBUG)
          } else {
            r.remove(InstanceFlags::DEBUG)
          }
        }
        r
      },
      backend_options: BackendOptions {
        dx12: Dx12BackendOptions {
          shader_compiler: if let Some(path) = config.dx_compiler_dll_path {
            Dx12Compiler::DynamicDxc { dxc_path: path }
          } else {
            Default::default()
          },
          ..Default::default()
        },
        ..Default::default()
      },
      memory_budget_thresholds: Default::default(),
      display: config.display,
    });
    Self::new(instance)
  }
}

impl GPU {
  /// create gpu without any compatible surface, mainly used in headless case.
  ///
  /// to create gpu with surface, create the instance by [GPUInstance::create] first, then create the
  /// raw surface by the instance, and use [GPU::new_with_instance] with the raw surface as the
  /// compatible surface, and finally create the [GPUSurface] by [GPUSurface::new]. The compatible
  /// surface is required in some backend, for example WebGL.
  pub async fn new(config: GPUCreateConfig) -> Result<Self, GPUCreateFailure> {
    let instance = GPUInstance::create(config.instance);
    Self::new_with_instance(instance, config.device, None).await
  }

  pub async fn new_with_instance(
    instance: GPUInstance,
    config: GPUDeviceConfig,
    compatible_surface: Option<&gpu::Surface<'_>>,
  ) -> Result<Self, GPUCreateFailure> {
    let adaptor = instance
      .request_adapter(&gpu::RequestAdapterOptions {
        power_preference: config.power_preference,
        compatible_surface,
        force_fallback_adapter: false,
      })
      .await
      .map_err(|err| {
        if cfg!(target_family = "wasm") && compatible_surface.is_none() {
          GPUCreateFailure::AdapterRequestFailedWithoutCompatibleSurface(err)
        } else {
          GPUCreateFailure::AdapterRequestFailed(err)
        }
      })?;

    let supported_features = adaptor.features();
    let supported_limits = adaptor.limits();

    if !config
      .minimal_required_limits
      .check_limits(&supported_limits)
    {
      // todo, list unsatisfied limits
      return Err(GPUCreateFailure::UnableToMeetLimitMinimalRequirement(
        Box::new(supported_limits),
      ));
    }
    if !supported_features.contains(config.minimal_required_features) {
      return Err(GPUCreateFailure::UnableToMeetFeatureMinimalRequirement(
        Box::new(config.minimal_required_features - supported_features),
      ));
    }

    let (device, queue) = adaptor
      .request_device(&gpu::DeviceDescriptor {
        label: None,
        required_features: supported_features,
        required_limits: supported_limits.clone(),
        memory_hints: MemoryHints::Performance,
        trace: wgpu_types::Trace::Off,
        experimental_features: unsafe { ExperimentalFeatures::enabled() },
      })
      .await?;

    let info = GPUInfo {
      adaptor_info: adaptor.get_info(),
      power_preference: config.power_preference,
      supported_features,
      supported_limits,
      downgrade_info: adaptor.get_downlevel_capabilities(),
    };

    let device = GPUDevice::new(device, info.clone(), config.default_shader_checks);
    let queue = GPUQueue::new(queue);

    Ok(Self {
      instance,
      adaptor,
      info: Arc::new(info),
      device,
      queue,
    })
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

  /// the target is usually `Arc<Window>` to make sure the surface is `'static`
  pub fn create_surface(
    &self,
    target: impl Into<gpu::SurfaceTarget<'static>>,
    init_resolution: Size,
  ) -> Result<GPUSurface, CreateSurfaceError> {
    let surface = self.instance.create_surface(target)?;
    Ok(GPUSurface::new(self, surface, init_resolution))
  }

  /// # Safety
  ///
  /// see [gpu::Instance::create_surface_unsafe]
  pub unsafe fn create_surface_unsafe(
    &self,
    target: gpu::SurfaceTargetUnsafe,
    init_resolution: Size,
  ) -> Result<GPUSurface, CreateSurfaceError> {
    let surface = unsafe { self.instance.create_surface_unsafe(target) }?;
    Ok(GPUSurface::new(self, surface, init_resolution))
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

/// this fn is to replace the usage of `TextureUsages::all()` because not every fmt support
/// `TextureUsages::STORAGE_ATOMIC` and this will cause validation error.
pub fn basic_texture_usages() -> TextureUsages {
  let mut full = TextureUsages::all();
  full.remove(TextureUsages::STORAGE_ATOMIC);
  full.remove(TextureUsages::TRANSIENT);
  full
}
