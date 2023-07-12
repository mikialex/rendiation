use crate::*;

pub trait HotAreaProvider {
  fn is_point_in(&self, point: UIPosition) -> bool;
}

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub states: &'a WindowState,
  pub fonts: &'a FontManager,
  pub texts: &'a mut TextCache,
  pub gpu: Arc<GPU>,
}
