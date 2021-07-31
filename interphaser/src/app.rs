use std::{
  rc::Rc,
  time::{Duration, Instant},
};

use rendiation_algebra::*;
use rendiation_texture::Size;
use rendiation_webgpu::*;
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
};

use crate::*;

pub struct Application<T> {
  event_loop: EventLoop<()>,
  app: ApplicationInner<T>,
}

pub struct ApplicationInner<T> {
  state: T,
  root: Box<dyn UIComponent<T>>,
  window_states: WindowState,
  root_size_changed: bool,
  ui_renderer: WebGPUxUIRenderer,
  fonts: FontManager,

  window: winit::window::Window,
  last_update_inst: Instant,
  swap_chain: GPUSwapChain,
  gpu: Rc<GPU>,
}

impl<T: 'static> Application<T> {
  pub async fn new(state: T, ui: impl UIComponent<T>) -> Self {
    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("viewer");
    let window = builder.build(&event_loop).unwrap();

    let initial_size = window.inner_size();
    let initial_size = (initial_size.width as f32, initial_size.height as f32);

    let (gpu, swap_chain) = GPU::new_with_swap_chain(&window).await;
    let gpu = Rc::new(gpu);

    let fonts = FontManager::new_with_fallback_system_font("Arial");

    let prefer_target_fmt = swap_chain.swap_chain_descriptor.format;
    let ui_renderer = WebGPUxUIRenderer::new(&gpu.device, prefer_target_fmt, &fonts);

    Self {
      event_loop,
      app: ApplicationInner {
        state,
        fonts,
        root: Box::new(ui),
        root_size_changed: true,
        window_states: WindowState::new(LayoutSize {
          width: initial_size.0,
          height: initial_size.1,
        }),
        ui_renderer,
        window,
        last_update_inst: Instant::now(),
        gpu,
        swap_chain,
      },
    }
  }

  pub fn run(self) {
    let mut app = self.app;
    self.event_loop.run(move |event, _, control_flow| {
      *control_flow = ControlFlow::Poll;

      app.event(&event);

      match &event {
        Event::MainEventsCleared => {
          // Clamp to some max framerate to avoid busy-looping too much
          // (we might be in wgpu::PresentMode::Mailbox, thus discarding superfluous frames)
          //
          // winit has window.current_monitor().video_modes() but that is a list of all full screen video modes.
          // So without extra dependencies it's a bit tricky to get the max refresh rate we can run the window on.
          // Therefore we just go with 60fps - sorry 120hz+ folks!
          let target_frametime = Duration::from_secs_f64(1.0 / 60.0);
          let time_since_last_frame = app.last_update_inst.elapsed();
          if time_since_last_frame >= target_frametime {
            app.window.request_redraw();
            app.last_update_inst = Instant::now();
          } else {
            *control_flow =
              ControlFlow::WaitUntil(Instant::now() + target_frametime - time_since_last_frame);
          }
        }
        Event::WindowEvent {
          event: WindowEvent::Resized(size),
          ..
        } => app.swap_chain.resize(
          Size::from_u32_pair_min_one((size.width, size.height)),
          &app.gpu.device,
        ),
        Event::WindowEvent { event, .. } => match event {
          WindowEvent::CloseRequested => {
            *control_flow = ControlFlow::Exit;
          }
          _ => {}
        },
        Event::RedrawRequested(_) => {
          let frame = app
            .swap_chain
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture!");

          app.render(frame);
        }
        _ => {}
      }
    });
  }
}

impl<T> ApplicationInner<T> {
  fn update(&mut self) {
    let mut ctx = UpdateCtx {
      time_stamp: 0,
      layout_changed: false,
    };
    self.root.update(&self.state, &mut ctx);

    let need_layout = ctx.layout_changed || self.root_size_changed;
    self.root_size_changed = false;
    if !need_layout {
      return;
    }

    let mut ctx = LayoutCtx { fonts: &self.fonts };

    self.root.layout(
      LayoutConstraint::from_max(self.window_states.size),
      &mut ctx,
    );
    self.root.set_position(UIPosition { x: 0., y: 0. })
  }

  fn render(&mut self, frame: SwapChainFrame) {
    self.update();

    let mut builder = PresentationBuilder {
      present: UIPresentation::new(),
    };
    self.root.render(&mut builder);
    builder.present.view_size = self.window_states.size;
    self.root.render(&mut builder);

    self.gpu.render(
      &mut WebGPUxUIRenderPass {
        fonts: &self.fonts,
        renderer: &mut self.ui_renderer,
        presentation: &builder.present,
      },
      &frame.output.view,
    )
  }

  fn event(&mut self, event: &winit::event::Event<()>) {
    let window_size = self.window_states.size;
    self.window_states.event(event);
    self.root_size_changed = window_size != self.window_states.size;
    let mut event = EventCtx {
      event,
      states: &self.window_states,
      gpu: self.gpu.clone(),
    };
    self.root.event(&mut self.state, &mut event)
  }
}
