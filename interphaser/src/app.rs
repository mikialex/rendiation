use std::{num::NonZeroUsize, sync::Arc};

use rendiation_texture::Size;
use winit::{event::*, event_loop::EventLoop};

use crate::*;

pub(crate) const TEXT_CACHE_INIT_SIZE: Size = Size {
  width: NonZeroUsize::new(512).unwrap(),
  height: NonZeroUsize::new(512).unwrap(),
};

pub async fn run_gui(ui: impl View + 'static, init_state: WindowSelfState) {
  let event_loop = EventLoop::new().unwrap();

  let mut window = Window::new(&event_loop, init_state.into(), futures::stream::pending());

  let mut renderer = WebGpuUIPresenter::new(&window.window).await;

  let mut app = Application::new(ui);

  event_loop.run(move |event, window_target| {
    window_target.set_control_flow(window.event(&event));

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
      Event::WindowEvent {
        event: WindowEvent::RedrawRequested,
        ..
      } => {
        let presentation = app.encode_presentation(window.window_states.size);
        renderer.render(&presentation, &app.fonts, &mut app.texts);
      }
      _ => {}
    }
  });
}

pub struct Application {
  root: Box<dyn View>,
  root_terminated: bool, // we could use something like fused view to express this
  any_changed: NotifyScope,
  event_filter: Box<dyn Fn(&PlatformEvent) -> bool>,
  fonts: FontManager,
  texts: TextCache,
}

impl Application {
  pub fn new(root: impl View + 'static) -> Self {
    let fonts = FontManager::new_with_default_font();
    let texts = TextCache::new_default_impl(TEXT_CACHE_INIT_SIZE);

    Self {
      root: Box::new(root),
      root_terminated: false,
      any_changed: Default::default(),
      fonts,
      texts,
      event_filter: Box::new(|_| true),
    }
  }

  fn event(&mut self, event: &winit::event::Event<()>, gpu: Arc<GPU>, window_states: &WindowState) {
    // if !(self.event_filter)(event) {// todo use with waker
    //   return;
    // }
    let mut event = EventCtx {
      event,
      states: window_states,
      fonts: &self.fonts,
      texts: &mut self.texts,
      event_filter: &mut self.event_filter,
      gpu,
    };
    self.root.event(&mut event);
    self.update();
  }

  fn update(&mut self) {
    self.any_changed.update_total(|cx| {
      if self.root_terminated {
        return;
      }
      println!("ui update");
      self.root_terminated = self
        .root
        .poll_until_pending_or_terminate_not_care_result(cx);
    });
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

    self.root.draw(&mut builder);

    builder.present
  }
}
