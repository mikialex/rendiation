use std::{
  num::NonZeroUsize,
  sync::{atomic::AtomicBool, Arc},
  task::Context,
};

use rendiation_texture::Size;
use webgpu::*;
use winit::{event::*, event_loop::EventLoop};

use crate::*;

pub(crate) const TEXT_CACHE_INIT_SIZE: Size = Size {
  width: NonZeroUsize::new(512).unwrap(),
  height: NonZeroUsize::new(512).unwrap(),
};

pub async fn run_gui(ui: impl View + 'static, init_state: WindowSelfState) {
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

pub struct Application {
  root: Box<dyn View>,
  any_changed: Arc<NotifyWaker>,
  fonts: FontManager,
  texts: TextCache,
}

struct NotifyWaker {
  inner: AtomicBool,
}

impl Default for NotifyWaker {
  fn default() -> Self {
    Self {
      inner: AtomicBool::new(true),
    }
  }
}

impl NotifyWaker {
  /// return if any changed contains
  pub fn check_reset_changed(&self) -> bool {
    self
      .inner
      .compare_exchange(
        true,
        false,
        std::sync::atomic::Ordering::SeqCst,
        std::sync::atomic::Ordering::SeqCst,
      )
      .is_ok()
  }
}

impl futures::task::ArcWake for NotifyWaker {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self
      .inner
      .fetch_or(true, std::sync::atomic::Ordering::SeqCst);
  }
}

impl Application {
  pub fn new(root: impl View + 'static) -> Self {
    let fonts = FontManager::new_with_default_font();
    let texts = TextCache::new_default_impl(TEXT_CACHE_INIT_SIZE);

    Self {
      root: Box::new(root),
      any_changed: Default::default(),
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
    self.root.request(&mut ViewRequest::Event(&mut event));
    self.update();
  }

  fn update(&mut self) {
    while self.any_changed.check_reset_changed() {
      println!("ui update");
      let waker = futures::task::waker_ref(&self.any_changed);
      let mut cx = Context::from_waker(&waker);
      do_updates_by(&mut self.root, &mut cx, |_| {});
    }
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
      .request(&mut ViewRequest::Layout(LayoutProtocol::DoLayout {
        constraint: LayoutConstraint::from_max(root_size),
        ctx: &mut ctx,
        output: &mut LayoutResult {
          size: root_size,
          baseline_offset: 0.,
        },
      }));
    self
      .root
      .request(&mut ViewRequest::Layout(LayoutProtocol::PositionAt(
        UIPosition { x: 0., y: 0. },
      )));

    // encoding render content
    let mut builder = PresentationBuilder::new(&self.fonts, &mut self.texts);
    builder.present.view_size = root_size;

    self.root.request(&mut ViewRequest::Encode(&mut builder));

    builder.present
  }
}
