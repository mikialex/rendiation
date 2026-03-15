use crate::*;

pub struct GPUPlatformConfig {
  pub preferred_backends: Option<Backends>,
  pub checks: ShaderRuntimeProtection,
  pub enable_backend_validation: Option<bool>,
  pub dx_compiler_dll_path: Option<String>,
}

impl GPUPlatformConfig {
  pub fn make_gpu_create_config<'a>(
    &self,
    surface_for_compatible_check_init: Option<(&'a (dyn SurfaceProvider + 'a), Size)>,
  ) -> GPUCreateConfig<'a> {
    GPUCreateConfig {
      surface_for_compatible_check_init,
      backends: self.preferred_backends.unwrap_or(Backends::all()),
      default_shader_checks: ShaderRuntimeChecks {
        bounds_checks: self.checks.bounds_checks,
        force_loop_bounding: self.checks.force_loop_bounding,
      },
      enable_backend_validation: self.enable_backend_validation,
      dx_compiler_dll_path: self.dx_compiler_dll_path.clone(),
      ..Default::default()
    }
  }
}

impl ViewerInitConfig {
  pub fn make_gpu_platform_config(&self) -> GPUPlatformConfig {
    GPUPlatformConfig {
      preferred_backends: self.init_only.wgpu_backend_select_override,
      checks: self.init_only.default_shader_protections,
      enable_backend_validation: self.init_only.enable_backend_validation,
      dx_compiler_dll_path: self.init_only.dx_compiler_dll_path.clone(),
    }
  }
}

pub struct WGPUAndSurface {
  pub surface: SurfaceWrapper,
  pub gpu: GPU,
}

impl WGPUAndSurface {
  pub async fn new<'a>(config: GPUCreateConfig<'a>) -> Self {
    let (gpu, surface) = GPU::new(config).await.unwrap();
    let surface: GPUSurface<'static> = unsafe { std::mem::transmute(surface.unwrap()) };
    let surface = SurfaceWrapper::new(surface);
    WGPUAndSurface { gpu, surface }
  }
}

#[derive(Clone)]
pub struct SurfaceWrapper {
  surface: Arc<RwLock<GPUSurface<'static>>>,
}

impl SurfaceWrapper {
  pub fn new(surface: GPUSurface<'static>) -> Self {
    Self {
      surface: Arc::new(RwLock::new(surface)),
    }
  }

  pub fn internal<R>(&self, v: impl FnOnce(&mut GPUSurface) -> R) -> R {
    let mut s = self.surface.write();
    v(&mut s)
  }

  pub fn set_size(&mut self, size: Size) {
    self.surface.write().set_size(size)
  }

  pub fn get_current_frame_with_render_target_view(
    &self,
    device: &GPUDevice,
  ) -> Result<(SurfaceTexture, RenderTargetView), SurfaceError> {
    self.surface.write().re_config_if_changed(device);
    self
      .surface
      .write()
      .get_current_frame_with_render_target_view(device)
  }
}
