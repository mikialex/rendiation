use std::{num::NonZeroUsize, sync::Arc, task::Context};

use rendiation_algebra::*;
use rendiation_texture::Size;
use webgpu::*;
use winit::{event::*, event_loop::EventLoop};

use crate::*;

const TEXT_CACHE_INIT_SIZE: Size = Size {
  width: NonZeroUsize::new(512).unwrap(),
  height: NonZeroUsize::new(512).unwrap(),
};

pub async fn run_gui(ui: impl Component + 'static, init_state: WindowSelfState) {
  let event_loop = EventLoop::new();

  let mut window = Window::new(&event_loop, init_state.into(), futures::stream::pending());

  let mut renderer = WebGpuUIPresenter::new(&window.window).await;

  let mut app = Application::new(ui);

  event_loop.run(move |event, _, control_flow| {
    *control_flow = window.event(&event);

    app.event(&event, renderer.gpu.clone(), &window.window_states);
    // todo window.poll_next_unpin(cx)

    match &event {
      Event::WindowEvent {
        event: WindowEvent::Resized(size),
        ..
      } => {
        let size = Size::from_u32_pair_min_one((size.width, size.height));
        renderer.resize(size);
      }
      Event::RedrawRequested(_) => {
        let presentation = app.encode_presentation(window.window_states.size);
        renderer.render(&presentation, &app.fonts, &mut app.texts);
      }
      _ => {}
    }
  });
}

pub trait UIPresenter {
  fn resize(&mut self, size: Size);
  fn render(&mut self, content: &UIPresentation, fonts: &FontManager, texts: &mut TextCache);
}

pub struct WebGpuUIPresenter {
  surface: GPUSurface,
  gpu: Arc<GPU>,
  ui_renderer: WebGPUxUIRenderer,
}

impl WebGpuUIPresenter {
  pub async fn new(window: &winit::window::Window) -> Self {
    let (gpu, surface) = GPU::new_with_surface(window).await;
    let gpu = Arc::new(gpu);

    let prefer_target_fmt = surface.config.format;
    let ui_renderer = WebGPUxUIRenderer::new(&gpu.device, prefer_target_fmt, TEXT_CACHE_INIT_SIZE);

    Self {
      surface,
      gpu,
      ui_renderer,
    }
  }
}

impl UIPresenter for WebGpuUIPresenter {
  fn resize(&mut self, size: Size) {
    self.surface.resize(size, &self.gpu.device);
  }

  fn render(&mut self, presentation: &UIPresentation, fonts: &FontManager, texts: &mut TextCache) {
    if let Ok((frame, view)) = self.surface.get_current_frame_with_render_target_view() {
      self.gpu.poll();

      let mut task = WebGPUxUIRenderTask {
        fonts,
        renderer: &mut self.ui_renderer,
        presentation,
      };

      let mut encoder = self.gpu.create_encoder();
      task.update(&self.gpu, &mut encoder, fonts, texts);

      let mut decs = RenderPassDescriptorOwned::default();
      decs.channels.push((
        webgpu::Operations {
          load: webgpu::LoadOp::Clear(webgpu::Color::WHITE),
          store: true,
        },
        view,
      ));
      {
        let mut pass = encoder.begin_render_pass(decs);
        task.setup_pass(&mut pass);
      }
      self.gpu.submit_encoder(encoder);

      frame.present();
    }
  }
}

pub struct Application {
  root: Box<dyn Component>,

  fonts: FontManager,
  texts: TextCache,
}

impl Application {
  pub fn new(root: impl Component + 'static) -> Self {
    let fonts = FontManager::new_with_default_font();
    let texts = TextCache::new_default_impl(TEXT_CACHE_INIT_SIZE);

    Self {
      root: Box::new(root),
      fonts,
      texts,
    }
  }

  fn event(&mut self, event: &winit::event::Event<()>, gpu: Arc<GPU>, window_states: &WindowState) {
    let mut event = EventCtx {
      event,
      states: window_states,
      fonts: &self.fonts,
      texts: &mut self.texts,
      gpu,
    };
    self.root.event(&mut event);
    // todo we should call update here
  }

  fn update(&mut self) {
    let waker = futures::task::noop_waker_ref();
    let mut cx = Context::from_waker(waker);
    do_updates_by(&mut self.root, &mut cx, |_| {});
  }

  fn encode_presentation(&mut self, root_size: UISize) -> UIPresentation {
    self.update();

    // recompute layout
    let mut ctx = LayoutCtx {
      fonts: &self.fonts,
      text: &self.texts,
    };

    self
      .root
      .layout(LayoutConstraint::from_max(root_size), &mut ctx);
    self.root.set_position(UIPosition { x: 0., y: 0. });

    // encoding render content
    let mut builder = PresentationBuilder::new(&self.fonts, &mut self.texts);
    builder.present.view_size = root_size;

    self.root.render(&mut builder);

    builder.present
  }
}
