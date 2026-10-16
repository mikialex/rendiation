use crate::*;

pub struct GPUPlatformConfig {
  pub preferred_backends: Option<Backends>,
  pub checks: ShaderRuntimeProtection,
  pub enable_backend_validation: Option<bool>,
  pub enable_hal_debug_info: Option<bool>,
  pub dx_compiler_dll_path: Option<String>,
  pub present_mode: PresentMode,
}

impl GPUPlatformConfig {
  pub fn make_gpu_create_config(&self) -> GPUCreateConfig {
    GPUCreateConfig {
      instance: GPUInstanceConfig {
        backends: self.preferred_backends.unwrap_or(Backends::all()),
        enable_backend_validation: self.enable_backend_validation,
        enable_debug_info: self.enable_hal_debug_info,
        dx_compiler_dll_path: self.dx_compiler_dll_path.clone(),
        ..Default::default()
      },
      device: GPUDeviceConfig {
        default_shader_checks: ShaderRuntimeChecks {
          bounds_checks: self.checks.bounds_checks,
          force_loop_bounding: self.checks.force_loop_bounding,
          ray_query_initialization_tracking: true,
          task_shader_dispatch_tracking: true,
          mesh_shader_primitive_indices_clamp: true,
        },
        ..Default::default()
      },
    }
  }
}

impl ViewerInitConfig {
  pub fn make_gpu_platform_config(&self) -> GPUPlatformConfig {
    GPUPlatformConfig {
      preferred_backends: self.init_only.wgpu_backend_select_override,
      checks: self.init_only.default_shader_protections,
      enable_backend_validation: self.init_only.enable_backend_validation,
      enable_hal_debug_info: self.init_only.enable_hal_debug_info,
      dx_compiler_dll_path: self.init_only.dx_compiler_dll_path.clone(),
      present_mode: self.present_mode,
    }
  }
}

pub struct WGPUAndSurface {
  pub surface: SurfaceWrapper,
  pub gpu: GPU,
}

impl WGPUAndSurface {
  /// the surface is created before the adapter and used as the compatible surface, which is
  /// required by WebGL backend.
  pub async fn new(
    config: GPUCreateConfig,
    target: impl Into<raw_gpu::SurfaceTarget<'static>>,
    init_resolution: Size,
  ) -> Self {
    let instance = GPUInstance::create(config.instance);
    let surface = instance
      .create_surface(target)
      .expect("failed to create surface");
    let gpu = GPU::new_with_instance(instance, config.device, Some(&surface))
      .await
      .expect("failed to create gpu");
    let surface = GPUSurface::new(&gpu, surface, init_resolution);
    WGPUAndSurface {
      surface: SurfaceWrapper::new(surface),
      gpu,
    }
  }
}

#[derive(Clone)]
pub struct SurfaceWrapper {
  surface: Arc<RwLock<GPUSurface>>,
}

impl SurfaceWrapper {
  pub fn new(surface: GPUSurface) -> Self {
    Self {
      surface: Arc::new(RwLock::new(surface)),
    }
  }

  pub fn is_hdr(&self) -> bool {
    self.read(|surface| surface.config().format == TextureFormat::Rgba16Float)
  }

  pub fn read<R>(&self, f: impl FnOnce(&GPUSurface) -> R) -> R {
    f(&self.surface.read())
  }

  pub fn write<R>(&self, f: impl FnOnce(&mut GPUSurface) -> R) -> R {
    f(&mut self.surface.write())
  }

  pub fn set_size(&self, size: Size) {
    self.surface.write().set_size(size)
  }

  pub fn set_present_mode_or_warn(&self, mode: PresentMode) {
    if let Err(err) = self.surface.write().set_present_mode(mode) {
      log::warn!("{err}, fallback to current present mode");
    }
  }

  /// return None if current frame should be skipped
  pub fn acquire_frame(&self, device: &GPUDevice) -> Result<Option<SurfaceFrame>, SurfaceError> {
    match self.surface.write().acquire_frame(device)? {
      SurfaceAcquire::Frame(frame) => {
        if frame.is_sub_optimal {
          log::warn!("surface frame target is suboptimal")
        }
        Ok(Some(frame))
      }
      SurfaceAcquire::Occluded => Ok(None),
      SurfaceAcquire::Timeout => {
        log::warn!("surface frame target failed to get(timeout)");
        Ok(None)
      }
    }
  }
}
