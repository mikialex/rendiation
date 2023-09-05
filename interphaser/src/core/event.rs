use crate::*;

pub type PlatformEvent<'a> = winit::event::Event<'a, ()>;

pub struct EventCtx<'a> {
  pub event: &'a PlatformEvent<'a>,
  pub states: &'a WindowState,
  pub fonts: &'a FontManager,
  pub texts: &'a mut TextCache,
  pub gpu: Arc<GPU>,
  pub event_filter: &'a mut dyn Fn(&PlatformEvent) -> bool,
}
