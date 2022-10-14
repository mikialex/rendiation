use crate::*;

pub trait SurfaceProvider {
  fn create_surface(&self, instance: &gpu::Instance) -> gpu::Surface;
  fn size(&self) -> Size;
}

impl SurfaceProvider for winit::window::Window {
  fn create_surface(&self, instance: &gpu::Instance) -> gpu::Surface {
    unsafe { instance.create_surface(self) }
  }

  fn size(&self) -> Size {
    let size = self.inner_size();
    Size::from_u32_pair_min_one((size.width, size.height))
  }
}

pub struct GPUSurface {
  pub surface: gpu::Surface,
  pub config: gpu::SurfaceConfiguration,
  pub present_mode_supported: Vec<gpu::PresentMode>,
  pub alpha_mode_supported: Vec<CompositeAlphaMode>,
  pub format_supported: Vec<TextureFormat>,
  pub size: Size,
}

impl GPUSurface {
  #[allow(clippy::or_fun_call)]
  pub fn new(
    adapter: &gpu::Adapter,
    device: &GPUDevice,
    surface: gpu::Surface,
    size: Size,
  ) -> Self {
    let formats = surface.get_supported_formats(adapter);
    let swapchain_format = formats
      .iter()
      .find(|&f| *f == gpu::TextureFormat::Rgba8UnormSrgb)
      .or(formats.first())
      .expect("could not find support formats in surface");

    let present_mode_supported = surface.get_supported_present_modes(adapter);
    let alpha_mode_supported = surface.get_supported_alpha_modes(adapter);
    let format_supported = surface.get_supported_formats(adapter);

    let config = gpu::SurfaceConfiguration {
      usage: gpu::TextureUsages::RENDER_ATTACHMENT,
      format: *swapchain_format,
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      present_mode: gpu::PresentMode::AutoVsync,
      alpha_mode: CompositeAlphaMode::Auto,
    };

    surface.configure(device, &config);

    Self {
      present_mode_supported,
      alpha_mode_supported,
      format_supported,
      surface,
      config,
      size,
    }
  }

  pub fn resize(&mut self, size: Size, device: &GPUDevice) {
    self.config.width = Into::<usize>::into(size.width) as u32;
    self.config.height = Into::<usize>::into(size.height) as u32;
    self.surface.configure(device, &self.config);
    self.size = size;
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
    let view = Rc::new(view);

    Ok((
      frame,
      RenderTargetView::SurfaceTexture {
        view,
        size: self.size,
        format: self.config.format,
        view_id: get_resource_view_guid(),
        invalidation_tokens: Default::default(),
      },
    ))
  }
}
