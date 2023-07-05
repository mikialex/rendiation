use std::sync::Arc;

use futures::Stream;
use rendiation_algebra::*;
use rendiation_texture::Size;
use webgpu::*;
use winit::{
  event::*,
  event_loop::{ControlFlow, EventLoop},
};

use crate::*;

pub async fn run_gui(
  ui: impl FnOnce(Box<dyn Stream<Item = Event<()>>>) -> Box<dyn UI>,
  config: WindowConfig,
) {
  let event_loop = EventLoop::new();
  let builder = winit::window::WindowBuilder::new();
  let window = builder.build(&event_loop).unwrap();

  window.set_title(&config.title);
  // let size = winit::dpi::LogicalSize {
  //   width: config.size.width as f64,
  //   height: config.size.height as f64,
  // };
  // window.set_inner_size(size);

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

  let initial_size = window.inner_size();
  let initial_size = (initial_size.width as f32, initial_size.height as f32);
  let device_pixel_ratio = window.scale_factor();

  let (gpu, surface) = GPU::new_with_surface(&window).await;
  let gpu = Arc::new(gpu);

  let fonts = FontManager::new_with_default_font();

  let text_cache_init_size = Size::from_usize_pair_min_one((512, 512));
  let texts = TextCache::new_default_impl(text_cache_init_size);

  let prefer_target_fmt = surface.config.format;
  let ui_renderer = WebGPUxUIRenderer::new(&gpu.device, prefer_target_fmt, text_cache_init_size);

  let mut app = Application {
    fonts,
    texts,
    root: todo!(),
    root_size_changed: true,
    window_states: WindowState::new(
      UISize {
        width: initial_size.0,
        height: initial_size.1,
      },
      device_pixel_ratio as f32,
    ),
    ui_renderer,
    window,
    last_update_inst: Instant::now(),
    init_inst: Instant::now(),
    view_may_changed: false,

    perf_info_last_frame: PerformanceInfo::new(0),
    current_frame_id: 1,
    current_perf: PerformanceInfo::new(1),
    gpu,
    surface,
  };

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Poll;

    app.event(&event);

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

        let time_since_last_frame = app.last_update_inst.elapsed();

        if time_since_last_frame >= target_frametime {
          app.last_update_inst = Instant::now();
          app.window.request_redraw();
        } else {
          #[cfg(not(target_arch = "wasm32"))]
          {
            *control_flow = ControlFlow::WaitUntil(
              Instant::now() + target_frametime.checked_sub(time_since_last_frame).unwrap(),
            );
          }
          #[cfg(target_arch = "wasm32")]
          app.window.request_redraw();
        }
      }
      Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => app.surface.resize(
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
        if let Ok((frame, view)) = app.surface.get_current_frame_with_render_target_view() {
          app.gpu.poll();
          app.update();
          app.render(&view);
          app.frame_end();
          frame.present();
        }
      }
      _ => {}
    }
  });
}

#[derive(Clone, PartialEq)]
pub struct WindowConfig {
  pub size: UISize,
  pub title: String,
  pub position: UIPosition,
}

pub trait UI: LayoutAble + Presentable + Component {}

pub struct Application {
  root: Box<dyn UI>,
  window_states: WindowState,
  root_size_changed: bool,
  ui_renderer: WebGPUxUIRenderer,
  fonts: FontManager,
  texts: TextCache,

  window: winit::window::Window,
  perf_info_last_frame: PerformanceInfo,
  current_frame_id: usize,
  current_perf: PerformanceInfo,

  view_may_changed: bool,
  init_inst: Instant,
  last_update_inst: Instant,
  surface: GPUSurface,
  gpu: Arc<GPU>,
}

impl Application {
  fn frame_end(&mut self) {
    self.current_frame_id += 1;
    self.perf_info_last_frame = self.current_perf;
    self.current_perf = PerformanceInfo::new(self.current_frame_id);
  }

  fn update(&mut self) {
    // let mut ctx = UpdateCtx {
    //   time_stamp: self.init_inst.elapsed(),
    //   layout_changed: false,
    //   fonts: &self.fonts,
    //   last_frame_perf_info: &self.perf_info_last_frame,
    // };
    // self.current_perf.update_time = time_measure(|| self.root.update(&self.state, &mut ctx));
    // self.view_may_changed = false;

    // self.current_perf.layout_time = time_measure(|| {
    //   let need_layout = ctx.layout_changed || self.root_size_changed;
    //   self.root_size_changed = false;
    //   if !need_layout {
    //     return;
    //   }

    //   let mut ctx = LayoutCtx {
    //     fonts: &self.fonts,
    //     text: &self.texts,
    //   };

    //   self.root.layout(
    //     LayoutConstraint::from_max(self.window_states.size),
    //     &mut ctx,
    //   );
    //   self.root.set_position(UIPosition { x: 0., y: 0. })
    // });
  }

  fn render(&mut self, frame: &RenderTargetView) {
    let mut builder = PresentationBuilder::new(&self.fonts, &mut self.texts);
    builder.present.view_size = self.window_states.size;

    self.current_perf.rendering_prepare_time = time_measure(|| self.root.render(&mut builder));

    self.current_perf.rendering_dispatch_time = time_measure(|| {
      let mut task = WebGPUxUIRenderTask {
        fonts: &self.fonts,
        renderer: &mut self.ui_renderer,
        presentation: &builder.present,
      };

      let mut encoder = self.gpu.create_encoder();
      task.update(&self.gpu, &mut encoder, &self.fonts, builder.texts);

      let mut decs = RenderPassDescriptorOwned::default();
      decs.channels.push((
        webgpu::Operations {
          load: webgpu::LoadOp::Clear(webgpu::Color::WHITE),
          store: true,
        },
        frame.clone(),
      ));
      {
        let mut pass = encoder.begin_render_pass(decs);
        task.setup_pass(&mut pass);
      }
      self.gpu.submit_encoder(encoder)
    });
  }

  fn event(&mut self, event: &winit::event::Event<()>) {
    let window_size = self.window_states.size;
    self.window_states.event(event);
    self.root_size_changed |= window_size != self.window_states.size;
    let mut event = EventCtx {
      event,
      custom_event: Default::default(),
      states: &self.window_states,
      fonts: &self.fonts,
      texts: &mut self.texts,
      gpu: self.gpu.clone(),
      view_may_changed: false,
    };
    self.root.event(&mut event);
    self.view_may_changed |= event.view_may_changed;
  }
}
