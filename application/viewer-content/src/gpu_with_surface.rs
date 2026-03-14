use crate::*;

pub struct WGPUAndSurface {
  pub surface: SurfaceWrapper,
  pub gpu: GPU,
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

  pub fn re_config_if_changed(&mut self, device: &GPUDevice) {
    self.surface.write().re_config_if_changed(device)
  }

  pub fn get_current_frame_with_render_target_view(
    &self,
    device: &GPUDevice,
  ) -> Result<(SurfaceTexture, RenderTargetView), SurfaceError> {
    self
      .surface
      .write()
      .get_current_frame_with_render_target_view(device)
  }
}
