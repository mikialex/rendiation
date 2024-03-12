use crate::*;

pub type PlatformEvent = winit::event::Event<()>;

pub struct EventCtx<'a> {
  pub event: &'a PlatformEvent,
  pub states: &'a WindowState,
  pub fonts: &'a FontManager,
  pub texts: &'a mut TextCache,
  pub gpu: Arc<GPU>,
  pub event_filter: &'a mut dyn Fn(&PlatformEvent) -> bool,
}
