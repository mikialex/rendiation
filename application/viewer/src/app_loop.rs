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
enum GPUOrGPUCreateFuture {
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

pub struct ApplicationCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub dyn_cx: &'a mut DynCx,
  pub window: &'a mut Window,
  pub input: &'a PlatformEventInput,
  pub gpu_and_surface: &'a WGPUAndSurface,
  pub draw_target_canvas: RenderTargetView,
}

pub type ApplicationDropCx = DynCx;

unsafe impl HooksCxLike for ApplicationCx<'_> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    self.memory
  }
  fn memory_ref(&self) -> &FunctionMemory {
    self.memory
  }
  fn flush(&mut self) {
    let drop_cx = &mut self.dyn_cx as *mut _ as *mut ();
    self.memory.flush(drop_cx);
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    // this is safe because user can not access previous retrieved state through returned self.
    let s = unsafe { std::mem::transmute_copy(&self) };

    let state = self
      .memory
      .expect_state_init(f, |_state: &mut T, _: &mut ApplicationDropCx| {});

    (s, state)
  }

  fn is_dynamic_stage(&self) -> bool {
    true
  }
}

struct WinitAppImpl {
  window: Option<WindowWithWGPUSurface>,
  memory: FunctionMemory,
  app_logic: Box<dyn Fn(&mut ApplicationCx)>,
  title: String,
  has_existed: bool,
  preferred_backends: Option<Backends>,
  checks: ShaderRuntimeChecks,
}

impl winit::application::ApplicationHandler for WinitAppImpl {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.window.get_or_insert_with(|| {
      #[allow(unused_mut)]
      let mut window_att = Window::default_attributes().with_title(&self.title);
      #[allow(unused_assignments)]
      let mut width = 0;
      #[allow(unused_assignments)]
      let mut height = 0;

      #[cfg(target_family = "wasm")]
      {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowAttributesExtWebSys;
        let canvas = web_sys::window()
          .unwrap()
          .document()
          .unwrap()
          .get_element_by_id("canvas")
          .unwrap()
          .dyn_into::<web_sys::HtmlCanvasElement>()
          .unwrap();
        window_att = window_att.with_canvas(Some(canvas.clone()));

        let ratio = web_sys::window().unwrap().device_pixel_ratio();

        // in wasm build, window.inner_size() is zero, so we have to fix
        width = (canvas.get_bounding_client_rect().width() * ratio).ceil() as u32;
        height = (canvas.get_bounding_client_rect().height() * ratio).ceil() as u32;
      }

      let window = event_loop.create_window(window_att).unwrap();
      log::info!("window created");
      window.request_redraw();
      let window = Box::pin(window);

      #[cfg(not(target_family = "wasm"))]
      {
        width = window.inner_size().width;
        height = window.inner_size().height;
      }

      log::info!("window physical size: {}x{}", width, height);
      #[allow(unused_mut)]
      let mut platform_states = PlatformEventInput::default();

      #[cfg(target_family = "wasm")]
      {
        let device = web_sys::window().unwrap().device_pixel_ratio() as f32;

        platform_states.window_state.device_pixel_ratio = device;
        platform_states.window_state.physical_size = (width as f32, height as f32);
      }

      let window_ref: &'static Window = unsafe { std::mem::transmute(window.as_ref()) };
      let config = GPUCreateConfig {
        surface_for_compatible_check_init: Some((
          window_ref,
          Size::from_u32_pair_min_one((width, height)),
        )),
        backends: self.preferred_backends.unwrap_or(Backends::all()),
        default_shader_checks: self.checks,
        ..Default::default()
      };

      #[cfg(feature = "support-webgl")]
      let config = if config.backends.contains(Backends::GL) {
        let mut config = config;
        config.minimal_required_limits = Limits::downlevel_webgl2_defaults();
        config
      } else {
        config
      };

      let gpu = GPUOrGPUCreateFuture::Creating(Box::pin(async {
        let (gpu, surface) = GPU::new(config).await.unwrap();
        let surface: GPUSurface<'static> = unsafe { std::mem::transmute(surface.unwrap()) };
        let surface = ApplicationWindowSurface::new(surface);
        WGPUAndSurface { gpu, surface }
      }));

      WindowWithWGPUSurface {
        window,
        gpu,
        platform_states,
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
    if self.has_existed {
      return;
    }

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
        match &event {
          WindowEvent::CloseRequested => {
            let mut cx = DynCx::default();
            self.memory.cleanup(&mut cx as *mut _ as *mut ());
            target.exit();
            self.has_existed = true;
          }
          WindowEvent::Resized(physical_size) => surface.set_size(Size::from_u32_pair_min_one((
            physical_size.width,
            physical_size.height,
          ))),
          WindowEvent::RedrawRequested => {
            surface.re_config_if_changed(&gpu.device);
            // when window resize to zero, the surface will be outdated.
            // but when should we deal with the surface lost case?
            if let Ok((output, canvas)) =
              surface.get_current_frame_with_render_target_view(&gpu.device)
            {
              let mut cx = DynCx::default();

              event_state.begin_frame();
              ApplicationCx {
                window,
                memory: &mut self.memory,
                dyn_cx: &mut cx,
                input: event_state,
                draw_target_canvas: canvas,
                gpu_and_surface,
              }
              .execute(|cx| (self.app_logic)(cx));
              event_state.end_frame();

              output.present();
            }
          }
          _ => {}
        };
      }

      // put the redraw request out of wgpu instance check
      // make sure it always requested
      if let WindowEvent::RedrawRequested = &event {
        window.request_redraw();
      }
    }
  }
}

pub fn run_application(
  preferred_backends: Option<Backends>,
  checks: ShaderRuntimeProtection,
  app_logic: impl Fn(&mut ApplicationCx) + 'static,
) {
  let event_loop = EventLoop::new().unwrap();

  // ControlFlow::Poll is unnecessary, and we have to specify wait on web target to avoid
  // excessive polling behavior
  event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

  let mut app = WinitAppImpl {
    window: None,
    memory: Default::default(),
    app_logic: Box::new(app_logic),
    title: "Rendiation Viewer".to_string(),
    has_existed: false,
    preferred_backends,
    checks: ShaderRuntimeChecks {
      bounds_checks: checks.bounds_checks,
      force_loop_bounding: checks.force_loop_bounding,
    },
  };

  event_loop.run_app(&mut app).unwrap();
}
