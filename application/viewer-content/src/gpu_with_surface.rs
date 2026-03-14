use crate::*;

pub struct WGPUAndSurface {
  pub surface: WindowSurfaceWrapper,
  pub gpu: GPU,
}

#[derive(Clone)]
pub struct WindowSurfaceWrapper {
  surface: Arc<RwLock<GPUSurface<'static>>>,
}

impl WindowSurfaceWrapper {
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

/// we use this to avoid block_on, which is not allowed in wasm
#[allow(clippy::large_enum_variant)]
pub enum GPUOrGPUCreateFuture {
  Created(WGPUAndSurface),
  Creating(Pin<Box<dyn Future<Output = WGPUAndSurface>>>),
}

impl GPUOrGPUCreateFuture {
  pub fn poll_gpu(&mut self) -> Option<&mut WGPUAndSurface> {
    match self {
      GPUOrGPUCreateFuture::Created(gpu) => Some(gpu),
      GPUOrGPUCreateFuture::Creating(future) => {
        noop_ctx!(ctx);
        if let Poll::Ready(gpu) = future.poll_unpin(ctx) {
          #[cfg(target_family = "wasm")]
          if gpu.gpu.info().adaptor_info.backend == Backend::Gl {
            log::warn!("selected backend is webgl, major performance issue may happen and features may missing");
          }

          *self = GPUOrGPUCreateFuture::Created(gpu);

          self.poll_gpu()
        } else {
          None
        }
      }
    }
  }
}
