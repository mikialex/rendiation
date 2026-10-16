use crate::*;

pub fn get_default_preferred_format(capabilities: &gpu::SurfaceCapabilities) -> gpu::TextureFormat {
  *capabilities
    .formats
    .iter()
    .find(|&f| *f == gpu::TextureFormat::Bgra8UnormSrgb) // prefer use srgb
    .or(capabilities.formats.first())
    .expect("none supported format exist in surface capabilities")
}

#[derive(thiserror::Error, Debug)]
pub enum SurfaceConfigUnsupported {
  #[error("surface format {0:?} is not supported")]
  Format(gpu::TextureFormat),
  #[error("surface present mode {0:?} is not supported")]
  PresentMode(gpu::PresentMode),
}

#[derive(thiserror::Error, Debug)]
pub enum SurfaceError {
  /// the raw surface must be recreated, see [GPUSurface::replace_raw_surface]
  #[error("surface lost")]
  Lost,
  #[error("surface validation error")]
  Validation,
  #[error("surface still outdated after reconfigure")]
  ReconfiguredStillOutdated,
}

pub enum SurfaceAcquire {
  Frame(SurfaceFrame),
  Occluded,
  Timeout,
}

pub struct SurfaceFrame {
  raw: gpu::SurfaceTexture,
  pub target: RenderTargetView,
  pub is_sub_optimal: bool,
}

impl SurfaceFrame {
  pub fn present(self) {
    self.raw.present();
  }
}

/// The swapchain abstraction.
///
/// The config changes are lazily applied in next [GPUSurface::acquire_frame] call.
///
/// Note, in WebGL backend, the gpu device is bound to the canvas of the compatible surface used
/// in gpu creation, so creating another surface for different canvas is not supported.
pub struct GPUSurface {
  surface: gpu::Surface<'static>,
  capabilities: gpu::SurfaceCapabilities,
  config: gpu::SurfaceConfiguration,
  require_reconfigure: bool,
}

impl GPUSurface {
  pub fn new(gpu: &GPU, surface: gpu::Surface<'static>, init_resolution: Size) -> Self {
    let capabilities = surface.get_capabilities(&gpu.adaptor);
    let (width, height) = init_resolution.into_u32();

    let config = gpu::SurfaceConfiguration {
      usage: gpu::TextureUsages::RENDER_ATTACHMENT,
      format: get_default_preferred_format(&capabilities),
      view_formats: vec![],
      width,
      height,
      present_mode: gpu::PresentMode::AutoVsync,
      alpha_mode: gpu::CompositeAlphaMode::Auto,
      desired_maximum_frame_latency: 2,
    };

    surface.configure(&gpu.device, &config);

    Self {
      surface,
      capabilities,
      config,
      require_reconfigure: false,
    }
  }

  /// Used to recover from [SurfaceError::Lost]. The current config is kept if still supported.
  pub fn replace_raw_surface(&mut self, gpu: &GPU, surface: gpu::Surface<'static>) {
    self.capabilities = surface.get_capabilities(&gpu.adaptor);
    if !self.is_format_supported(self.config.format) {
      self.config.format = get_default_preferred_format(&self.capabilities);
    }
    if !self.is_present_mode_supported(self.config.present_mode) {
      self.config.present_mode = gpu::PresentMode::AutoVsync;
    }
    self.surface = surface;
    self.require_reconfigure = true;
  }

  pub fn capabilities(&self) -> &gpu::SurfaceCapabilities {
    &self.capabilities
  }

  pub fn config(&self) -> &gpu::SurfaceConfiguration {
    &self.config
  }

  pub fn size(&self) -> Size {
    Size::from_u32_pair_min_one((self.config.width, self.config.height))
  }

  pub fn set_size(&mut self, size: Size) {
    let (width, height) = size.into_u32();
    if self.config.width != width || self.config.height != height {
      self.config.width = width;
      self.config.height = height;
      self.require_reconfigure = true;
    }
  }

  pub fn is_format_supported(&self, format: gpu::TextureFormat) -> bool {
    self.capabilities.formats.contains(&format)
  }

  pub fn set_format(&mut self, format: gpu::TextureFormat) -> Result<(), SurfaceConfigUnsupported> {
    if !self.is_format_supported(format) {
      return Err(SurfaceConfigUnsupported::Format(format));
    }
    if self.config.format != format {
      self.config.format = format;
      self.require_reconfigure = true;
    }
    Ok(())
  }

  pub fn is_present_mode_supported(&self, mode: gpu::PresentMode) -> bool {
    // the auto modes always have fallback
    matches!(
      mode,
      gpu::PresentMode::AutoVsync | gpu::PresentMode::AutoNoVsync
    ) || self.capabilities.present_modes.contains(&mode)
  }

  pub fn set_present_mode(
    &mut self,
    mode: gpu::PresentMode,
  ) -> Result<(), SurfaceConfigUnsupported> {
    if !self.is_present_mode_supported(mode) {
      return Err(SurfaceConfigUnsupported::PresentMode(mode));
    }
    if self.config.present_mode != mode {
      self.config.present_mode = mode;
      self.require_reconfigure = true;
    }
    Ok(())
  }

  fn configure(&mut self, device: &GPUDevice) {
    self.surface.configure(device, &self.config);
    self.require_reconfigure = false;
  }

  pub fn acquire_frame(&mut self, device: &GPUDevice) -> Result<SurfaceAcquire, SurfaceError> {
    use wgpu::CurrentSurfaceTexture;

    if self.require_reconfigure {
      self.configure(device);
    }

    let mut is_sub_optimal = false;

    let frame = match self.surface.get_current_texture() {
      CurrentSurfaceTexture::Success(frame) => frame,
      CurrentSurfaceTexture::Timeout => return Ok(SurfaceAcquire::Timeout),
      CurrentSurfaceTexture::Occluded => return Ok(SurfaceAcquire::Occluded),
      // If the surface is outdated or suboptimal, reconfigure and retry.
      CurrentSurfaceTexture::Suboptimal(_) | CurrentSurfaceTexture::Outdated => {
        self.configure(device);
        match self.surface.get_current_texture() {
          CurrentSurfaceTexture::Success(frame) => frame,
          CurrentSurfaceTexture::Suboptimal(frame) => {
            is_sub_optimal = true;
            frame
          }
          CurrentSurfaceTexture::Timeout => return Ok(SurfaceAcquire::Timeout),
          CurrentSurfaceTexture::Occluded => return Ok(SurfaceAcquire::Occluded),
          CurrentSurfaceTexture::Outdated => return Err(SurfaceError::ReconfiguredStillOutdated),
          CurrentSurfaceTexture::Lost => return Err(SurfaceError::Lost),
          CurrentSurfaceTexture::Validation => return Err(SurfaceError::Validation),
        }
      }
      CurrentSurfaceTexture::Validation => return Err(SurfaceError::Validation),
      CurrentSurfaceTexture::Lost => return Err(SurfaceError::Lost),
    };

    let view = frame
      .texture
      .create_view(&gpu::TextureViewDescriptor::default());
    let view = Arc::new(view);

    let view_id = create_resource_view_guid();

    let target = RenderTargetView::SurfaceTexture {
      view,
      size: Size::from_u32_pair_min_one((frame.texture.width(), frame.texture.height())),
      format: frame.texture.format(),
      view_id,
      binding_dropper: Arc::new(device.get_binding_cache().create_dropper(view_id)),
    };

    Ok(SurfaceAcquire::Frame(SurfaceFrame {
      raw: frame,
      target,
      is_sub_optimal,
    }))
  }
}
