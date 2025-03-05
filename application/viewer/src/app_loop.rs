use std::sync::Arc;

use parking_lot::RwLock;

use crate::*;

struct WindowWithWGPUSurface {
  window: Pin<Box<winit::window::Window>>,
  platform_states: PlatformEventInput,
  gpu: GPUOrGPUCreateFuture,
}

pub struct WGPUAndSurface {
  pub surface: ApplicationWindowSurface,
  pub gpu: GPU,
}

#[derive(Clone)]
pub struct ApplicationWindowSurface {
  surface: Arc<RwLock<GPUSurface<'static>>>,
}

impl ApplicationWindowSurface {
  pub fn new(surface: GPUSurface<'static>) -> Self {
    Self {
      surface: Arc::new(RwLock::new(surface)),
    }
  }

  pub fn internal(&self, v: impl FnOnce(&mut GPUSurface)) {
    let mut s = self.surface.write();
    v(&mut s);
  }

  pub fn set_size(&mut self, size: Size) {
    self.surface.write().set_size(size)
  }

  pub fn re_config_if_changed(&mut self, device: &GPUDevice) {
    self.surface.write().re_config_if_changed(device)
  }

  pub fn get_current_frame_with_render_target_view(
    &self,
  ) -> Result<(SurfaceTexture, RenderTargetView), SurfaceError> {
    self
      .surface
      .write()
      .get_current_frame_with_render_target_view()
  }
}

/// we use this to avoid block_on, which is not allowed in wasm
enum GPUOrGPUCreateFuture {
  Created(WGPUAndSurface),
  Creating(Box<dyn Future<Output = WGPUAndSurface> + Unpin>),
}

impl GPUOrGPUCreateFuture {
  pub fn poll_gpu(&mut self) -> Option<&mut WGPUAndSurface> {
    match self {
      GPUOrGPUCreateFuture::Created(gpu) => Some(gpu),
      GPUOrGPUCreateFuture::Creating(future) => {
        noop_ctx!(ctx);
        if let Poll::Ready(gpu) = future.poll_unpin(ctx) {
          *self = GPUOrGPUCreateFuture::Created(gpu);
          self.poll_gpu()
        } else {
          None
        }
      }
    }
  }
}

struct WinitAppImpl<T> {
  window: Option<WindowWithWGPUSurface>,
  root: T,
  title: String,
}

impl<T: Widget> winit::application::ApplicationHandler for WinitAppImpl<T> {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.window.get_or_insert_with(|| {
      let window = event_loop
        .create_window(Window::default_attributes().with_title(&self.title))
        .unwrap();
      window.request_redraw();
      let window = Box::pin(window);

      let width = window.inner_size().width;
      let height = window.inner_size().height;

      let window_ref: &'static Window = unsafe { std::mem::transmute(window.as_ref()) };
      let config = GPUCreateConfig {
        surface_for_compatible_check_init: Some((
          window_ref,
          Size::from_u32_pair_min_one((width, height)),
        )),
        ..Default::default()
      };

      let gpu = GPUOrGPUCreateFuture::Creating(Box::new(Box::pin(async {
        let (gpu, surface) = GPU::new(config).await.unwrap();
        let surface: GPUSurface<'static> = unsafe { std::mem::transmute(surface.unwrap()) };
        let surface = ApplicationWindowSurface::new(surface);
        WGPUAndSurface { gpu, surface }
      })));

      WindowWithWGPUSurface {
        window,
        gpu,
        platform_states: Default::default(),
      }
    });
  }

  fn device_event(
    &mut self,
    _: &winit::event_loop::ActiveEventLoop,
    device_id: winit::event::DeviceId,
    event: winit::event::DeviceEvent,
  ) {
    if let Some(WindowWithWGPUSurface {
      platform_states: event_state,
      ..
    }) = &mut self.window
    {
      event_state.queue_event(Event::DeviceEvent {
        event: event.clone(),
        device_id,
      });
    }
  }

  fn window_event(
    &mut self,
    target: &winit::event_loop::ActiveEventLoop,
    _: winit::window::WindowId,
    event: WindowEvent,
  ) {
    if let Some(WindowWithWGPUSurface {
      window,
      platform_states: event_state,
      gpu,
    }) = &mut self.window
    {
      // safety depend on that we don't replace the entire window in our application logic
      let window = unsafe { window.as_mut().get_unchecked_mut() };
      if let Some(mut gpu_and_surface) = gpu.poll_gpu() {
        let WGPUAndSurface { surface, gpu } = &mut gpu_and_surface;
        event_state.queue_event(Event::WindowEvent {
          event: event.clone(),
          window_id: window.id(),
        });
        match event {
          WindowEvent::CloseRequested => {
            target.exit();
          }
          WindowEvent::Resized(physical_size) => surface.set_size(Size::from_u32_pair_min_one((
            physical_size.width,
            physical_size.height,
          ))),
          WindowEvent::RedrawRequested => {
            surface.re_config_if_changed(&gpu.device);
            // when window resize to zero, the surface will be outdated.
            // but when should we deal with the surface lost case?
            if let Ok((output, mut canvas)) = surface.get_current_frame_with_render_target_view() {
              let mut cx = DynCx::default();

              event_state.begin_frame();
              cx.scoped_cx(window, |cx| {
                cx.scoped_cx(event_state, |cx| {
                  cx.scoped_cx(gpu_and_surface, |cx| {
                    cx.scoped_cx(&mut canvas, |cx| {
                      self.root.update_state(cx);
                    });
                  });
                });
              });

              event_state.end_frame();
              cx.scoped_cx(window, |cx| {
                cx.scoped_cx(event_state, |cx| {
                  cx.scoped_cx(gpu_and_surface, |cx| {
                    cx.scoped_cx(&mut canvas, |cx| {
                      self.root.update_view(cx);
                    });
                  });
                });
              });

              output.present();
            }

            window.request_redraw();
          }
          _ => {}
        };
      }
    }
  }
}

pub fn run_application<T: Widget>(app: T) {
  let event_loop = EventLoop::new().unwrap();

  // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
  // dispatched any events. This is ideal for games and similar applications.
  event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

  let mut app = WinitAppImpl {
    window: None,
    root: app,
    title: "Rendiation Viewer".to_string(),
  };

  event_loop.run_app(&mut app).unwrap();
}
