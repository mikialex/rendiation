use crate::*;

struct WindowWithWGPUSurface {
  window: Arc<winit::window::Window>,
  gpu: GPUOrGPUCreateFuture,
}

pub struct ApplicationCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub dyn_cx: &'a mut DynCx,
  pub window: &'a Window,
  pub input: &'a WindowEventStates,
  /// current window
  pub gpu_and_surface: &'a WGPUAndSurface,
  pub surface_id: u32,
  pub draw_target_canvas: RenderTargetView,
}

impl<'a> ApplicationCx<'a> {
  pub fn use_state_init<T>(
    &mut self,
    init: impl FnOnce() -> T,
    drop_from_cx: fn(&mut T, &mut DynCx),
  ) -> (&mut Self, &mut T)
  where
    T: Any,
  {
    let this = self as *mut Self;
    let state = unsafe { (*this).memory.expect_state_init(init, drop_from_cx) };
    // SAFETY: this is derived from a valid &mut self; state points into bump-allocated heap
    // memory inside memory, not into the struct itself, so no aliased &mut is created.
    let this = unsafe { &mut *this };
    (this, state)
  }
}

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
    let this = self as *mut Self;
    let state = unsafe {
      (*this)
        .memory
        .expect_state_init(f, |_state: &mut T, _: &mut DynCx| {})
    };
    // SAFETY: this is derived from a valid &mut self; state points into bump-allocated heap
    // memory inside memory, not into the struct itself, so no aliased &mut is created.
    let this = unsafe { &mut *this };
    (this, state)
  }

  fn is_dynamic_stage(&self) -> bool {
    true
  }
}

struct WinitAppImpl {
  windows: FastHashMap<winit::window::WindowId, (u32, WindowWithWGPUSurface)>,
  platform_states: PlatformEventInput,
  memory: FunctionMemory,
  app_logic: Box<dyn Fn(&mut ApplicationCx)>,
  title: String,
  has_exited: bool,
  config: GPUPlatformConfig,
}

impl winit::application::ApplicationHandler for WinitAppImpl {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if self.windows.is_empty() {
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
      let window_id = window.id();
      log::info!("window created");
      window.request_redraw();

      #[cfg(not(target_family = "wasm"))]
      {
        width = window.inner_size().width;
        height = window.inner_size().height;
      }

      log::info!("window physical size: {}x{}", width, height);

      #[allow(unused)]
      let window_state = self
        .platform_states
        .get_or_init_window_state(window_id, &window);

      #[cfg(target_family = "wasm")]
      {
        // todo, is this necessary?
        window_state.window_state.physical_size = (width as f32, height as f32);
      }

      let window = Arc::new(window);

      let mut config = self.config.make_gpu_create_config();
      config.instance.display = Some(Box::new(event_loop.owned_display_handle()));

      #[cfg(feature = "support-webgl")]
      if config.instance.backends.contains(Backends::GL) {
        config.device.minimal_required_limits = Limits::downlevel_webgl2_defaults();
      }

      let init_resolution = Size::from_u32_pair_min_one((width, height));
      let present_mode = self.config.present_mode;
      let surface_target = window.clone();
      let fut = async move {
        let gpu_and_surface = WGPUAndSurface::new(config, surface_target, init_resolution).await;
        gpu_and_surface
          .surface
          .set_present_mode_or_warn(present_mode);
        gpu_and_surface
      };
      let gpu = GPUOrGPUCreateFuture::Creating(Box::pin(fut));

      let window = WindowWithWGPUSurface { window, gpu };

      self.windows.insert(window_id, (0, window));
    };
  }

  fn device_event(
    &mut self,
    _: &winit::event_loop::ActiveEventLoop,
    device_id: winit::event::DeviceId,
    event: winit::event::DeviceEvent,
  ) {
    self.platform_states.input_device_event(&event, device_id);
  }

  fn window_event(
    &mut self,
    target: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    if self.has_exited {
      return;
    }

    if let WindowEvent::Destroyed = event {
      self.platform_states.remove_window(window_id);
    }

    if let Some((surface_id, WindowWithWGPUSurface { window, gpu })) =
      &mut self.windows.get_mut(&window_id)
    {
      let window_handle = window.clone();
      let window = &**window;
      if let Some(mut gpu_and_surface) = gpu.poll_gpu() {
        let WGPUAndSurface { surface, gpu } = &mut gpu_and_surface;
        let window_state = self
          .platform_states
          .get_or_init_window_state(window_id, window);
        // on macos, when window closed, this event will be pushed continuously.
        // so we directly filter it to avoid queue unbounded this kind of events
        if event != WindowEvent::RedrawRequested {
          window_state.queue_event(Event::WindowEvent {
            event: event.clone(),
            window_id: window.id(),
          });
        }

        match &event {
          WindowEvent::CloseRequested => {
            let mut cx = DynCx::default();
            self.memory.cleanup(&mut cx as *mut _ as *mut ());
            target.exit();
            self.has_exited = true;
          }
          WindowEvent::Resized(physical_size) => surface.set_size(Size::from_u32_pair_min_one((
            physical_size.width,
            physical_size.height,
          ))),
          WindowEvent::RedrawRequested => {
            let frame = match surface.acquire_frame(&gpu.device) {
              Ok(frame) => frame,
              Err(SurfaceError::Lost) => {
                log::warn!("surface lost, try recreate");
                match gpu.instance.create_surface(window_handle) {
                  Ok(raw) => surface.write(|s| s.replace_raw_surface(gpu, raw)),
                  Err(err) => log::error!("failed to recreate surface: {err}"),
                }
                None
              }
              Err(err) => {
                log::error!("surface error: {err}");
                None
              }
            };

            if let Some(frame) = frame {
              let mut cx = DynCx::default();

              let window_state = self
                .platform_states
                .get_or_init_window_state(window_id, window);
              window_state.begin_frame();

              ApplicationCx {
                window,
                memory: &mut self.memory,
                dyn_cx: &mut cx,
                input: window_state,
                draw_target_canvas: frame.target.clone(),
                gpu_and_surface,
                surface_id: *surface_id,
              }
              .execute(|cx| (self.app_logic)(cx));

              window_state.end_frame();

              frame.present();
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
            log::warn!(
              "selected backend is webgl, major performance issue may happen and features may missing"
            );
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

pub fn run_application(
  config: GPUPlatformConfig,
  app_logic: impl Fn(&mut ApplicationCx) + 'static,
) {
  let event_loop = EventLoop::new().unwrap();

  // ControlFlow::Poll is unnecessary, and we have to specify wait on web target to avoid
  // excessive polling behavior
  event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

  let mut app = WinitAppImpl {
    windows: Default::default(),
    memory: Default::default(),
    app_logic: Box::new(app_logic),
    title: "Rendiation Viewer".to_string(),
    has_exited: false,
    config,
    platform_states: Default::default(),
  };

  event_loop.run_app(&mut app).unwrap();
}
