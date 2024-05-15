use crate::*;

#[derive(Default)]
pub struct WidgetGroup {
  children: Vec<Box<dyn Widget>>,
}

impl WidgetGroup {
  pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
    self.children.push(Box::new(child));
    self
  }
}

impl Widget for WidgetGroup {
  fn update_view(&mut self, cx: &mut StateCx) {
    for c in &mut self.children {
      c.update_view(cx)
    }
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    for c in &mut self.children {
      c.update_state(cx)
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    for child in &mut self.children {
      child.clean_up(cx)
    }
  }
}
