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
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    for c in &mut self.children {
      c.update_view(cx)
    }
  }
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    for c in &mut self.children {
      c.update_state(cx)
    }
  }
}

pub struct UIArray<T> {
  children: Vec<Box<dyn View>>,
  marker: std::marker::PhantomData<T>,
}

impl<T> View for UIArray<T> {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    // cx.state
    // cx.state.register_state(v)
    for c in &mut self.children {
      c.update_view(cx)
    }
  }
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    for c in &mut self.children {
      c.update_state(cx)
    }
  }
}
