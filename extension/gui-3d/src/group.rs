use crate::*;

#[derive(Default)]
pub struct UIGroup {
  children: Vec<Box<dyn View>>,
}

impl UIGroup {
  pub fn with_child(mut self, child: impl View + 'static) -> Self {
    self.children.push(Box::new(child));
    self
  }
}

impl View for UIGroup {
  fn update_view(&mut self, model: &mut ViewStateStore) {
    for c in &mut self.children {
      c.update_view(model)
    }
  }
  fn update_state(&mut self, cx: &mut View3dCtx) {
    for c in &mut self.children {
      c.update_state(cx)
    }
  }
}
