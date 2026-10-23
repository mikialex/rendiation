use crate::*;

/// The swapchain abstraction.
///
/// The config changes are lazily applied in next [GPUSurface::acquire_frame] call.
///
/// Note, in WebGL backend, the gpu device is bound to the canvas of the compatible surface used
/// in gpu creation, so creating another surface for different canvas is not supported.
pub struct GPUSurface {
  surface: gpu::Surface<'static>,
  capabilities: gpu::SurfaceCapabilities,
  support_view_formats: bool,
  config: gpu::SurfaceConfiguration,
  require_reconfigure: bool,
}

impl GPUSurface {
  pub fn new(gpu: &GPU, surface: gpu::Surface<'static>, init_resolution: Size) -> Self {
    let capabilities = surface.get_capabilities(&gpu.adaptor);
    let support_view_formats = gpu
      .info
      .downgrade_info
      .flags
      .contains(DownlevelFlags::SURFACE_VIEW_FORMATS);
    let (width, height) = init_resolution.into_u32();

    let render_format = select_default_render_format(&capabilities, support_view_formats);
    let (format, view_format) =
      resolve_render_format(&capabilities, support_view_formats, render_format).unwrap();

    let config = gpu::SurfaceConfiguration {
      usage: gpu::TextureUsages::RENDER_ATTACHMENT,
      format,
      view_formats: view_format.into_iter().collect(),
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
      support_view_formats,
      config,
      require_reconfigure: false,
    }
  }

  /// Used to recover from [SurfaceError::Lost]. The current config is kept if still supported.
  pub fn replace_raw_surface(&mut self, gpu: &GPU, surface: gpu::Surface<'static>) {
    self.capabilities = surface.get_capabilities(&gpu.adaptor);
    if !self.is_format_supported(self.render_format()) {
      self.apply_render_format(self.default_render_format());
    }
    if !self.is_present_mode_supported(self.config.present_mode) {
      self.config.present_mode = gpu::PresentMode::AutoVsync;
    }
    self.surface = surface;
    self.require_reconfigure = true;
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

    let format = self.render_format();
    let view = frame.texture.create_view(&gpu::TextureViewDescriptor {
      format: Some(format),
      ..Default::default()
    });
    let size = Size::from_u32_pair_min_one((frame.texture.width(), frame.texture.height()));
    let target = RenderTargetView::from_surface_view(device, view, size, format);

    Ok(SurfaceAcquire::Frame(SurfaceFrame {
      raw: frame,
      target,
      is_sub_optimal,
    }))
  }

  fn configure(&mut self, device: &GPUDevice) {
    self.surface.configure(device, &self.config);
    self.require_reconfigure = false;
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

  /// the format of the acquired frame's render target. it's may differ from the surface storage
  /// format(config.format) when the srgb view of none srgb surface format is used.
  pub fn render_format(&self) -> gpu::TextureFormat {
    self
      .config
      .view_formats
      .first()
      .copied()
      .unwrap_or(self.config.format)
  }

  pub fn default_render_format(&self) -> gpu::TextureFormat {
    select_default_render_format(&self.capabilities, self.support_view_formats)
  }

  pub fn is_format_supported(&self, render_format: gpu::TextureFormat) -> bool {
    resolve_render_format(&self.capabilities, self.support_view_formats, render_format).is_some()
  }

  /// the format is the render format, see [GPUSurface::render_format]
  pub fn set_format(
    &mut self,
    render_format: gpu::TextureFormat,
  ) -> Result<(), SurfaceConfigUnsupported> {
    if self.apply_render_format(render_format) {
      Ok(())
    } else {
      Err(SurfaceConfigUnsupported::Format(render_format))
    }
  }

  /// return false if not supported
  fn apply_render_format(&mut self, render_format: gpu::TextureFormat) -> bool {
    let Some((format, view_format)) =
      resolve_render_format(&self.capabilities, self.support_view_formats, render_format)
    else {
      return false;
    };
    if self.config.format != format || self.config.view_formats.first().copied() != view_format {
      self.config.format = format;
      self.config.view_formats = view_format.into_iter().collect();
      self.require_reconfigure = true;
    }
    true
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

#[derive(thiserror::Error, Debug)]
pub enum SurfaceConfigUnsupported {
  #[error("surface format {0:?} is not supported")]
  Format(gpu::TextureFormat),
  #[error("surface present mode {0:?} is not supported")]
  PresentMode(gpu::PresentMode),
}

/// prefer srgb format, use the srgb view of none srgb format if it's not directly supported.
fn select_default_render_format(
  capabilities: &gpu::SurfaceCapabilities,
  support_view_formats: bool,
) -> gpu::TextureFormat {
  let formats = &capabilities.formats;
  formats
    .iter()
    .copied()
    .find(|f| f.is_srgb())
    .or_else(|| {
      formats.iter().map(|f| f.add_srgb_suffix()).find(|f| {
        f.is_srgb() && resolve_render_format(capabilities, support_view_formats, *f).is_some()
      })
    })
    .or(formats.first().copied())
    .expect("none supported format exist in surface capabilities")
}

/// return the surface storage format and the optional view format to get the expected render format
fn resolve_render_format(
  capabilities: &gpu::SurfaceCapabilities,
  support_view_formats: bool,
  render_format: gpu::TextureFormat,
) -> Option<(gpu::TextureFormat, Option<gpu::TextureFormat>)> {
  if capabilities.formats.contains(&render_format) {
    return Some((render_format, None));
  }
  // for example the WebGPU canvas only support none srgb formats, but allow srgb view
  let storage_format = render_format.remove_srgb_suffix();
  (support_view_formats
    && render_format.is_srgb()
    && capabilities.formats.contains(&storage_format))
  .then_some((storage_format, Some(render_format)))
}
