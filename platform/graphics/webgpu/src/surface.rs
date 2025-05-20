use crate::*;

pub trait SurfaceProvider {
  fn create_surface<'a>(
    &'a self,
    instance: &gpu::Instance,
  ) -> Result<gpu::Surface<'a>, CreateSurfaceError>;
  fn size(&self) -> Size;
}

impl SurfaceProvider for winit::window::Window {
  fn create_surface<'a>(
    &'a self,
    instance: &gpu::Instance,
  ) -> Result<gpu::Surface<'a>, CreateSurfaceError> {
    instance.create_surface(self)
  }

  fn size(&self) -> Size {
    let size = self.inner_size();
    Size::from_u32_pair_min_one((size.width, size.height))
  }
}

#[cfg(target_arch = "wasm32")]
impl SurfaceProvider for web_sys::HtmlCanvasElement {
  fn create_surface<'a>(
    &'a self,
    instance: &wgpu::Instance,
  ) -> Result<wgpu::Surface<'a>, CreateSurfaceError> {
    let surface_target = wgpu::SurfaceTarget::Canvas(self.clone());
    instance.create_surface(surface_target)
  }

  fn size(&self) -> Size {
    Size::from_u32_pair_min_one((self.width(), self.height()))
  }
}

pub struct GPUSurface<'a> {
  surface: gpu::Surface<'a>,
  synced_config: gpu::SurfaceConfiguration,
  pub config: gpu::SurfaceConfiguration,
  capabilities: gpu::SurfaceCapabilities,
}

pub fn get_default_preferred_format(capabilities: &gpu::SurfaceCapabilities) -> gpu::TextureFormat {
  *capabilities
    .formats
    .iter()
    .find(|&f| *f == gpu::TextureFormat::Bgra8UnormSrgb) // prefer use srgb
    .or(capabilities.formats.first())
    .expect("none supported format exist in surface capabilities")
}

impl<'a> GPUSurface<'a> {
  #[allow(clippy::or_fun_call)]
  pub(crate) fn new(
    adapter: &gpu::Adapter,
    device: &GPUDevice,
    surface: gpu::Surface<'a>,
    init_resolution: Size,
  ) -> Self {
    let capabilities = surface.get_capabilities(adapter);
    let swapchain_format = get_default_preferred_format(&capabilities);

    let config = gpu::SurfaceConfiguration {
      usage: gpu::TextureUsages::RENDER_ATTACHMENT,
      format: swapchain_format,
      view_formats: vec![],
      width: Into::<usize>::into(init_resolution.width) as u32,
      height: Into::<usize>::into(init_resolution.height) as u32,
      present_mode: if std::env::consts::OS == "windows" {
        // disable vsync on windows in default config due to unreasonable high latency
        gpu::PresentMode::AutoNoVsync
      } else {
        gpu::PresentMode::AutoVsync
      },
      alpha_mode: gpu::CompositeAlphaMode::Auto,
      desired_maximum_frame_latency: 2,
    };

    surface.configure(device, &config);

    Self {
      synced_config: config.clone(),
      capabilities,
      surface,
      config,
    }
  }

  pub fn capabilities(&self) -> &gpu::SurfaceCapabilities {
    &self.capabilities
  }

  pub fn size(&self) -> Size {
    Size::from_u32_pair_min_one((self.config.width, self.config.height))
  }

  pub fn set_size(&mut self, size: Size) {
    self.config.width = Into::<usize>::into(size.width) as u32;
    self.config.height = Into::<usize>::into(size.height) as u32;
  }

  pub fn re_config_if_changed(&mut self, device: &GPUDevice) {
    if self.config == self.synced_config {
      return;
    }
    self.surface.configure(device, &self.config);
    self.synced_config = self.config.clone();
  }

  pub fn get_current_frame(&self) -> Result<gpu::SurfaceTexture, gpu::SurfaceError> {
    self.surface.get_current_texture()
  }

  pub fn get_current_frame_with_render_target_view(
    &self,
  ) -> Result<(gpu::SurfaceTexture, RenderTargetView), gpu::SurfaceError> {
    let frame = self.get_current_frame()?;

    let view = frame
      .texture
      .create_view(&gpu::TextureViewDescriptor::default());
    let view = Arc::new(view);

    Ok((
      frame,
      RenderTargetView::SurfaceTexture {
        view,
        size: self.size(),
        format: self.config.format,
        view_id: get_resource_view_guid(),
        bindgroup_holder: Default::default(),
      },
    ))
  }
}
