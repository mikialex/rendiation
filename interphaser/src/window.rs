use std::ops::DerefMut;

use winit::{event::WindowEvent, event_loop::ControlFlow};

use crate::*;

#[derive(Clone, PartialEq)]
pub struct WindowSelfState {
  pub size: UISize,
  pub title: String,
  pub position: UIPosition,
}

impl WindowSelfState {
  fn apply(&self, window: &winit::window::Window) {
    window.set_title(&self.title);
    window.set_outer_position(winit::dpi::Position::Logical(winit::dpi::LogicalPosition {
      x: self.position.x as f64,
      y: self.position.y as f64,
    }));
    // window.set_inner_size(winit::dpi::LogicalSize {
    //   width: self.size.width as f64,
    //   height: self.size.height as f64,
    // })
  }
}

pub struct Window {
  pub(crate) window_states: WindowState,
  pub(crate) window: winit::window::Window,
  states: BoxedUnpinFusedStream<WindowSelfState>,
  last_update_inst: Instant,
}

impl Window {
  pub fn new(
    event_loop: &winit::event_loop::EventLoop<()>,
    init_states: Option<WindowSelfState>,
    states_updater: impl Stream<Item = WindowSelfState> + Unpin + 'static,
  ) -> Self {
    let builder = winit::window::WindowBuilder::new();
    let window = builder.build(event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
      use winit::platform::web::WindowExtWebSys;
      web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| {
          body
            .append_child(&web_sys::Element::from(window.canvas()))
            .ok()
        })
        .expect("couldn't append canvas to document body");
    }

    if let Some(init) = init_states {
      init.apply(&window)
    }

    let initial_size = window.inner_size();
    let initial_size = (initial_size.width as f32, initial_size.height as f32);
    let device_pixel_ratio = window.scale_factor();
    let window_states = WindowState::new(
      UISize {
        width: initial_size.0,
        height: initial_size.1,
      },
      device_pixel_ratio as f32,
    );

    Self {
      window,
      window_states,
      states: Box::new(states_updater.fuse()),
      last_update_inst: Instant::now(),
    }
  }

  pub fn event(&mut self, event: &Event<()>) -> ControlFlow {
    self.window_states.event(event);

    match &event {
      Event::MainEventsCleared => {
        // Clamp to some max framerate to avoid busy-looping too much
        // (we might be in webgpu::PresentMode::Mailbox, thus discarding superfluous frames)
        //
        // winit has window.current_monitor().video_modes() but that is a list of all full screen
        // video modes. So without extra dependencies it's a bit tricky to get the max
        // refresh rate we can run the window on. Therefore we just go with 60fps -
        // sorry 120hz+ folks!
        let target_frametime = Duration::from_secs_f64(1.0 / 60.0);

        let time_since_last_frame = self.last_update_inst.elapsed();

        if time_since_last_frame >= target_frametime {
          self.last_update_inst = Instant::now();
          self.window.request_redraw();
        } else {
          #[cfg(not(target_arch = "wasm32"))]
          {
            return ControlFlow::WaitUntil(
              Instant::now() + target_frametime.checked_sub(time_since_last_frame).unwrap(),
            );
          }
          #[cfg(target_arch = "wasm32")]
          self.window.request_redraw();
        }
      }
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::CloseRequested => {
          return ControlFlow::Exit;
        }
        _ => {}
      },
      _ => {}
    }
    ControlFlow::Poll
  }
}

impl Stream for Window {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.deref_mut();
    this
      .states
      .poll_until_pending(cx, |new| new.apply(&this.window));
    Poll::Pending
  }
}
