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
  pub size: Size,
}

impl GPUSurface {
  pub fn new(
    adapter: &gpu::Adapter,
    device: &GPUDevice,
    surface: gpu::Surface,
    size: Size,
  ) -> Self {
    let swapchain_format = surface
      .get_preferred_format(adapter)
      .unwrap_or(gpu::TextureFormat::Rgba8UnormSrgb);

    let config = gpu::SurfaceConfiguration {
      usage: gpu::TextureUsages::RENDER_ATTACHMENT,
      format: swapchain_format,
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      present_mode: gpu::PresentMode::Mailbox,
    };

    surface.configure(device, &config);

    Self {
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
        view: view.clone(),
        size: self.size,
        format: self.config.format,
        view_id: get_resource_view_guid(),
        invalidation_tokens: Default::default(),
      },
    ))
  }
}
