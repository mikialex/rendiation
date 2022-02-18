use crate::*;

pub trait SurfaceProvider {
  fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface;
  fn size(&self) -> Size;
}

impl SurfaceProvider for winit::window::Window {
  fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface {
    unsafe { instance.create_surface(self) }
  }

  fn size(&self) -> Size {
    let size = self.inner_size();
    Size::from_u32_pair_min_one((size.width, size.height))
  }
}

pub struct GPUSurface {
  pub surface: wgpu::Surface,
  pub config: wgpu::SurfaceConfiguration,
  pub size: Size,
}

impl GPUSurface {
  pub fn new(
    adapter: &wgpu::Adapter,
    device: &GPUDevice,
    surface: wgpu::Surface,
    size: Size,
  ) -> Self {
    let swapchain_format = surface
      .get_preferred_format(adapter)
      .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);

    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: swapchain_format,
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      present_mode: wgpu::PresentMode::Mailbox,
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

  pub fn get_current_frame(&mut self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
    self.surface.get_current_texture()
  }
}
